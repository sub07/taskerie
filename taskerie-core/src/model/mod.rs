use indexmap::IndexMap;
use task::Task;

pub mod action;
pub mod task;

#[derive(Debug)]
pub struct TaskerieContext {
    pub tasks: IndexMap<String, Task>,
}

#[derive(Default)]
pub struct ParamContext {
    pub params: IndexMap<String, String>,
}

impl ParamContext {
    pub fn has(&self, param_name: &str) -> bool {
        self.params.contains_key(param_name)
    }

    pub fn set(&mut self, param_name: &str, value: &str) {
        self.params
            .insert(param_name.to_string(), value.to_string());
    }

    pub fn get(&self, param_name: &str) -> Option<&String> {
        self.params.get(param_name)
    }
}
