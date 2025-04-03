use indexmap::IndexMap;

use super::action::Action;

#[derive(Debug)]
pub struct Task {
    pub actions: Vec<Action>,
    pub on_success: Vec<Action>,
    pub on_failure: Vec<Action>,
    pub params: IndexMap<String, Param>,
}

#[derive(Debug)]
pub struct Param {
    pub default: Option<String>,
}

impl Param {
    pub fn is_required(&self) -> bool {
        self.default.is_none()
    }
}
