use std::{
    path::Path,
    sync::{Arc, mpsc},
    thread,
};

use anyhow::Context;
use taskerie_core::{message::ExecutionMessage, model::ParamContext};

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let path = if cfg!(debug_assertions) {
        Path::new("taskerie.example.yaml")
    } else {
        Path::new("taskerie.yaml")
    };
    let mut taskerie = Arc::new(taskerie_core::load(path).with_context(|| path.display())?);

    let reload = "\u{2699}  Reload taskerie".to_string();
    let exit = "\u{2699}  Exit".to_string();

    loop {
        let mut task_names = taskerie.get_all_standalone_task_names();
        task_names.push(reload.clone());
        task_names.push(exit.clone());

        let selected_task = inquire::Select::new("Select a task to execute", task_names)
            .with_page_size(999)
            .prompt()?;

        if selected_task == reload {
            debug_assert_eq!(Arc::strong_count(&taskerie), 1);
            taskerie = Arc::new(taskerie_core::load(path).with_context(|| path.display())?);
            println!("Sucessfully reloaded");
            continue;
        }

        if selected_task == exit {
            break;
        }

        let (tx, rx) = mpsc::channel();
        let executor_taskerie = taskerie.clone();
        let executor_selected_task = selected_task.clone();

        let executor_thread = thread::spawn(move || {
            executor_taskerie.run_task_by_name(
                executor_selected_task,
                &mut ParamContext::default(),
                &tx,
            )?;
            anyhow::Ok(())
        });

        for message in rx {
            match message {
                ExecutionMessage::MissingRequiredTaskParameter { parameter_name } => {
                    println!(
                        "Parameter '{parameter_name}' is undefined and has no default value provided"
                    );
                }
                ExecutionMessage::WorkingDirectoryNotFound { path } => {
                    println!("\u{274C} Requested working directory \"{path}\" not found");
                }
                ExecutionMessage::AboutToRunCommand {
                    command,
                    working_directory,
                } => {
                    println!("\u{231C} {working_directory}> {command}");
                }
                ExecutionMessage::CommandFailed => {
                    println!("\u{231E}\u{274C}");
                }
                ExecutionMessage::CommandSucceeded => {
                    println!("\u{231E}\u{2705}");
                }
                ExecutionMessage::CommandOutput { output } => {
                    println!("\u{23B8}{output}");
                }
            }
        }

        if let Err(e) = executor_thread.join().unwrap() {
            eprintln!("Error executing task: {e}");
        }
    }

    Ok(())
}
