use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Deserialize, Debug)]

pub struct Root {
    pub tasks: IndexMap<String, Task>,
}

#[derive(Deserialize, Debug)]
pub struct Task {
    pub actions: Vec<String>,
    #[serde(default)]
    pub on_failure: Vec<String>,
    #[serde(default)]
    pub on_success: Vec<String>,
    #[serde(default)]
    pub params: IndexMap<String, Param>,
}

#[derive(Deserialize, Debug)]
pub struct Param {
    pub default: Option<String>,
}
