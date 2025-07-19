use indexmap::IndexMap;

use crate::model::InterpolatedString;

use super::action::Action;

#[derive(Debug)]
pub struct Task {
    pub working_directory: Option<InterpolatedString>,
    pub actions: Vec<Action>,
    pub on_success: Vec<Action>,
    pub on_failure: Vec<Action>,
    pub params: IndexMap<String, Param>,
}

impl Task {
    #[must_use]
    pub fn has_no_default_params(&self) -> bool {
        !self.params.is_empty() && self.params.values().any(|param| param.default.is_none())
    }
}

#[derive(Debug)]
pub struct Param {
    pub default: Option<String>,
}

impl Param {
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.default.is_none()
    }
}
