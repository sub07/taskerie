use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug)]

pub struct Root {
    pub tasks: HashMap<String, Task>,
}

#[derive(Deserialize, Debug)]
pub struct Task {
    pub actions: Vec<String>,
    #[serde(default)]
    pub on_failure: Vec<String>,
    #[serde(default)]
    pub on_success: Vec<String>,
    #[serde(default)]
    pub args: HashMap<String, Arg>,
}

#[derive(Deserialize, Debug)]
pub struct Arg {
    #[serde(rename = "type")]
    pub ty: String,
    pub default: Option<String>,
}
