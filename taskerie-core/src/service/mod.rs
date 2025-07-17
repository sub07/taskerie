use std::{
    io::Write,
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

use anyhow::{anyhow, bail};

use crate::model::{self, InterpolatedString, ParamContext, TaskerieContext};

pub mod action;
pub mod interpolated_string;
pub mod task_parser;

impl TaskerieContext {
    #[must_use]
    pub fn get_all_task_names(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter(|task| !task.1.has_no_default_params())
            .map(|task| task.0.clone())
            .collect()
    }

    #[must_use]
    pub fn get_task_by_name(&self, name: &str) -> Option<&model::Task> {
        self.tasks.get(name)
    }

    pub fn run_task(
        &self,
        task: &model::task::Task,
        param_context: &mut ParamContext,
    ) -> anyhow::Result<ExitStatus> {
        for (name, param) in &task.params {
            if param_context.has(name) {
                continue;
            }
            if let Some(default_value) = &param.default {
                param_context.set(name, default_value);
            } else {
                bail!("Task param {name} doesn't have any value and has no default value provided");
            }
        }

        for action in &task.actions {
            let action_status =
                self.run_action(action, task.working_directory.as_deref(), param_context)?;
            if !action_status.success() {
                for failure_action in &task.on_failure {
                    if !self
                        .run_action(
                            failure_action,
                            task.working_directory.as_deref(),
                            param_context,
                        )?
                        .success()
                    {
                        eprintln!("Failure action failed");
                    }
                }
                return Ok(action_status);
            }
        }

        for success_action in &task.on_success {
            if !self
                .run_action(
                    success_action,
                    task.working_directory.as_deref(),
                    param_context,
                )?
                .success()
            {
                eprintln!("Success action failed");
            }
        }

        Ok(ExitStatus::default())
    }

    fn run_action(
        &self,
        action: &model::action::Action,
        working_directory: Option<&Path>,
        param_context: &ParamContext,
    ) -> anyhow::Result<ExitStatus> {
        match action {
            model::action::Action::Command(command) => {
                run_command(command, working_directory, param_context)
            }
            model::action::Action::TaskCall(task_call) => {
                self.run_task_from_action(task_call, param_context)
            }
        }
    }

    fn run_task_from_action(
        &self,
        task_call: &model::action::TaskCall,
        param_context: &ParamContext,
    ) -> anyhow::Result<ExitStatus> {
        let task = self
            .get_task_by_name(&task_call.name)
            .ok_or_else(|| anyhow!("Task {} is not defined", task_call.name))?;
        let mut task_param_context = ParamContext::default();
        for (param_name, param_value) in &task_call.params {
            task_param_context.set(param_name, &param_value.render(param_context)?);
        }
        self.run_task(task, &mut task_param_context)
    }
}

fn run_command(
    command: &InterpolatedString,
    working_directory: Option<&Path>,
    param_context: &ParamContext,
) -> anyhow::Result<ExitStatus> {
    let mut process = Command::new("pwsh")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg("-")
        .current_dir(working_directory.unwrap_or_else(|| Path::new("./")))
        .stdin(Stdio::piped())
        .spawn()?;

    let command = command.render(param_context)?;

    println!("> {command}");

    writeln!(
        process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("Could not get powershell stdin for command {}", command))?,
        "{command}"
    )?;

    Ok(process.wait()?)
}
