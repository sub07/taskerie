use std::{
    io::Write,
    process::{Command, ExitStatus, Stdio},
};

use action::{render_argument_parts, render_argument_parts_in};
use anyhow::{anyhow, bail};

use crate::model::{self, ParamContext, TaskerieContext};

pub mod action;
pub mod action_parser;
pub mod task_parser;

impl TaskerieContext {
    pub fn get_all_task_names(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    pub fn get_task_by_name(&self, name: &str) -> Option<&model::task::Task> {
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
            model::action::Action::Task(task) => self.run_action_target_task(task, param_context),
        }
    }

    fn run_action_target_task(
        &self,
        target_task: &model::action::TargetTask,
        param_context: &mut ParamContext,
    ) -> anyhow::Result<ExitStatus> {
        let task = self
            .get_task_by_name(&target_task.name)
            .ok_or(anyhow!("Task {} is not defined", target_task.name))?;
        let mut task_param_context = ParamContext::default();
        for (param_name, param_value) in target_task.params.iter() {
            task_param_context.set(
                param_name,
                &render_argument_parts(param_value, param_context)?,
            );
        }
        self.run_task(task, &mut task_param_context)
    }
}

fn run_command(
    command: &model::action::Command,
    param_context: &mut ParamContext,
) -> anyhow::Result<ExitStatus> {
    let mut child = Command::new("pwsh")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()?;

    let mut shell_command = String::new();

    shell_command.push_str(&command.name);

    for parts in &command.arguments {
        shell_command.push(' ');
        shell_command.push('"');
        render_argument_parts_in(parts, param_context, &mut shell_command)?;
        shell_command.push('"');
    }

    writeln!(
        child.stdin.as_mut().ok_or(anyhow!(
            "Could not get powershell stdin for command {}",
            command.name
        ))?,
        "{shell_command}"
    )?;

    Ok(child.wait()?)
}
