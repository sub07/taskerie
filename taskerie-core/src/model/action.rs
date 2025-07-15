use indexmap::IndexMap;

use crate::model::InterpolatedString;

#[derive(PartialEq, Debug)]
pub struct TaskCall {
    pub name: String,
    pub params: IndexMap<String, InterpolatedString>,
}

#[derive(PartialEq, Debug)]
pub enum Action {
    TaskCall(TaskCall),
    Command(InterpolatedString),
}
