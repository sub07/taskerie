use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::mpsc,
};

use anyhow::{anyhow, bail};
use subprocess::{Exec, ExitStatus, Redirection};

use crate::{
    message::ExecutionMessage,
    model::{self, InterpolatedString, ParamContext, TaskerieContext},
};

pub mod action;
pub mod interpolated_string;
pub mod task_parser;

impl TaskerieContext {
    #[must_use]
    pub fn get_all_standalone_task_names(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter(|task| task.1.is_standalone())
            .map(|task| task.0.clone())
            .collect()
    }

    #[must_use]
    fn get_task_by_name<S: AsRef<str>>(&self, name: S) -> Option<&model::Task> {
        self.tasks.get(name.as_ref())
    }

    pub fn run_task_by_name<S: AsRef<str>>(
        &self,
        name: S,
        param_context: &mut ParamContext,
        execution_message_sender: &mpsc::Sender<ExecutionMessage>,
    ) -> anyhow::Result<ExitStatus> {
        if let Some(task) = self.get_task_by_name(name) {
            self.run_task(task, param_context, execution_message_sender)
        } else {
            bail!("Task not found");
        }
    }

    fn run_task(
        &self,
        task: &model::task::Task,
        param_context: &mut ParamContext,
        execution_message_sender: &mpsc::Sender<ExecutionMessage>,
    ) -> anyhow::Result<ExitStatus> {
        for (name, param) in &task.params {
            if param_context.has(name) {
                continue;
            }
            if let Some(default_value) = &param.default {
                param_context.set(name, default_value);
            } else {
                execution_message_sender.send(ExecutionMessage::MissingRequiredTaskParameter {
                    parameter_name: name.clone(),
                })?;
                return Ok(ExitStatus::Undetermined);
            }
        }

        for action in &task.actions {
            let status = self.run_action(
                action,
                task.working_directory.as_ref(),
                param_context,
                execution_message_sender,
            )?;

            if !status.success() {
                break;
            }
        }

        Ok(ExitStatus::Exited(0))
    }

    fn run_action(
        &self,
        action: &model::action::Action,
        working_directory: Option<&InterpolatedString>,
        param_context: &ParamContext,
        execution_message_sender: &mpsc::Sender<ExecutionMessage>,
    ) -> anyhow::Result<ExitStatus> {
        match action {
            model::action::Action::Command(command) => run_command(
                command,
                working_directory,
                param_context,
                execution_message_sender,
            ),
            model::action::Action::TaskCall(task_call) => {
                self.run_task_from_action(task_call, param_context, execution_message_sender)
            }
        }
    }

    fn run_task_from_action(
        &self,
        task_call: &model::action::TaskCall,
        param_context: &ParamContext,
        execution_message_sender: &mpsc::Sender<ExecutionMessage>,
    ) -> anyhow::Result<ExitStatus> {
        let task = self
            .get_task_by_name(&task_call.name)
            .ok_or_else(|| anyhow!("Task {} is not defined", task_call.name))?;
        let mut task_param_context = ParamContext::default();
        for (param_name, param_value) in &task_call.params {
            task_param_context.set(param_name, &param_value.render(param_context)?);
        }
        self.run_task(task, &mut task_param_context, execution_message_sender)
    }
}

fn run_command(
    command: &InterpolatedString,
    working_directory: Option<&InterpolatedString>,
    param_context: &ParamContext,
    execution_message_sender: &mpsc::Sender<ExecutionMessage>,
) -> anyhow::Result<ExitStatus> {
    let current_dir = working_directory
        .map(|dir| dir.render(param_context))
        .transpose()?
        .unwrap_or_else(|| "./".into());

    let Ok(current_dir) = PathBuf::from(&*current_dir).canonicalize() else {
        execution_message_sender.send(ExecutionMessage::WorkingDirectoryNotFound {
            path: current_dir.clone().into_owned(),
        })?;
        return Ok(ExitStatus::Undetermined);
    };
    let current_dir_str = current_dir.display().to_string();
    let command = command.render(param_context)?;

    execution_message_sender.send(ExecutionMessage::AboutToRunCommand {
        command: command.clone().into_owned(),
        working_directory: current_dir_str.clone(),
    })?;

    let mut process = Exec::cmd("pwsh")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(command.clone().into_owned())
        .cwd(current_dir)
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Merge)
        .popen()?;

    let output_reader = BufReader::new(
        process
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow!("Could not get powershell stdout {}", command))?,
    );

    for line in output_reader.lines() {
        execution_message_sender.send(ExecutionMessage::CommandOutput { output: line? })?;
    }

    if process.wait()?.success() {
        execution_message_sender.send(ExecutionMessage::CommandSucceeded)?;
    } else {
        execution_message_sender.send(ExecutionMessage::CommandFailed)?;
    }

    Ok(process
        .exit_status()
        .expect("Exit status is available because the process is done already"))
}
