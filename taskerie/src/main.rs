use anyhow::Context;
use taskerie_core::model::ParamContext;

fn main() -> anyhow::Result<()> {
    let path = if cfg!(debug_assertions) {
        "taskerie.example.yaml".to_string()
    } else {
        "taskerie.yaml".to_string()
    };
    let taskerie = taskerie_core::load(path.clone()).with_context(|| path)?;

    loop {
        let mut task_names = taskerie.get_all_task_names();
        task_names.push("Exit".into());

        let selected_task =
            inquire::Select::new("Select a task to execute", task_names).prompt()?;

        if selected_task == "Exit" {
            break;
        }

        let task = taskerie
            .get_task_by_name(&selected_task)
            .expect("Come from list, so should exist");

        taskerie.run_task(task, &mut ParamContext::default())?;
    }

    Ok(())
}
