use anyhow::Context;
use taskerie_core::model::ParamContext;

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let path = if cfg!(debug_assertions) {
        "taskerie.example.yaml".to_string()
    } else {
        "taskerie.yaml".to_string()
    };
    let mut taskerie = taskerie_core::load(path.clone()).with_context(|| path.clone())?;

    let reload = "\u{2699}  Reload taskerie".to_string();
    let exit = "\u{2699}  Exit".to_string();

    loop {
        let mut task_names = taskerie.get_all_task_names();
        task_names.push(reload.clone());
        task_names.push(exit.clone());

        let selected_task =
            inquire::Select::new("Select a task to execute", task_names).prompt()?;

        if selected_task == reload {
            taskerie = taskerie_core::load(path.clone()).with_context(|| path.clone())?;
            println!("Sucessfully reloaded");
            continue;
        }

        if selected_task == exit {
            break;
        }

        let task = taskerie
            .get_task_by_name(&selected_task)
            .expect("Come from list, so should exist");

        taskerie.run_task(task, &mut ParamContext::default())?;
    }

    Ok(())
}
