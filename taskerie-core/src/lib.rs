mod config;
pub mod model;
mod service;

use std::{fs, path::Path};

use config::Root;
use indexmap::IndexMap;
use model::TaskerieContext;

pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<TaskerieContext> {
    let config = serde_norway::from_str::<Root>(&fs::read_to_string(path)?)?;

    let tasks = config
        .tasks
        .into_iter()
        .map(|(name, task)| task.try_into().map(|t: model::task::Task| (name, t)))
        .collect::<anyhow::Result<IndexMap<_, _>>>()?;

    Ok(TaskerieContext { tasks })
}
