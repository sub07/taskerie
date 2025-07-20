use indexmap::IndexMap;

use crate::model::InterpolatedString;

use super::action::Action;

#[derive(Debug)]
pub struct Task {
    pub working_directory: Option<InterpolatedString>,
    pub actions: Vec<Action>,
    pub params: IndexMap<String, Param>,
}

impl Task {
    /// Check whether the task can be executed without any additional parameters.
    #[must_use]
    pub fn is_standalone(&self) -> bool {
        self.params.is_empty() || self.params.values().all(|param| param.default.is_some())
    }
}

#[derive(Debug)]
pub struct Param {
    pub default: Option<String>,
}
