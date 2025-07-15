use std::{
    io::Write,
    process::{Command, ExitStatus, Stdio},
};

use anyhow::{anyhow, bail};

use crate::model::{self, InterpolatedString, ParamContext, TaskerieContext};

pub mod action;
pub mod interpolated_string;
pub mod task_parser;

impl TaskerieContext {
    pub fn get_all_task_names(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    pub fn get_task_by_name(&self, name: &str) -> Option<&model::Task> {
        self.tasks.get(name)
    }

    pub fn run_task(
        &self,
        task: &model::task::Task,
        param_context: &mut ParamContext,
    ) -> anyhow::Result<ExitStatus> {
        for (name, param) in task.params.iter() {
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
            let action_status = self.run_action(action, param_context)?;
            if !action_status.success() {
                for failure_action in &task.on_failure {
                    if !self.run_action(failure_action, param_context)?.success() {
                        eprintln!("Failure action failed");
                    }
                }
                return Ok(action_status);
            }
        }

        for success_action in &task.on_success {
            if !self.run_action(success_action, param_context)?.success() {
                eprintln!("Success action failed");
            }
        }

        Ok(ExitStatus::default())
    }

    fn run_action(
        &self,
        action: &model::action::Action,
        param_context: &mut ParamContext,
    ) -> anyhow::Result<ExitStatus> {
        match action {
            model::action::Action::Command(command) => run_command(command, param_context),
            model::action::Action::TaskCall(task_call) => {
                self.run_task_from_action(task_call, param_context)
            }
        }
    }

    fn run_task_from_action(
        &self,
        task_call: &model::action::TaskCall,
        param_context: &mut ParamContext,
    ) -> anyhow::Result<ExitStatus> {
        let task = self
            .get_task_by_name(&task_call.name)
            .ok_or(anyhow!("Task {} is not defined", task_call.name))?;
        let mut task_param_context = ParamContext::default();
        for (param_name, param_value) in task_call.params.iter() {
            task_param_context.set(param_name, &param_value.render(param_context)?);
        }
        self.run_task(task, &mut task_param_context)
    }
}

fn run_command(
    command: &InterpolatedString,
    param_context: &mut ParamContext,
) -> anyhow::Result<ExitStatus> {
    let mut child = Command::new("pwsh")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()?;

    let command = command.render(param_context)?;

    log::debug!("executed command: {command}");

    writeln!(
        child.stdin.as_mut().ok_or(anyhow!(
            "Could not get powershell stdin for command {}",
            command
        ))?,
        "{command}"
    )?;

    Ok(child.wait()?)
}
