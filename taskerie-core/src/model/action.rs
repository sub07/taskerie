use indexmap::IndexMap;

#[derive(PartialEq, Debug)]
pub enum ArgumentPart {
    Literal(String),
    Variable(String),
}

#[derive(PartialEq, Debug)]
pub struct Command {
    pub name: String,
    pub arguments: Vec<Vec<ArgumentPart>>,
}

#[derive(PartialEq, Debug)]
pub struct TargetTask {
    pub name: String,
    pub params: IndexMap<String, Vec<ArgumentPart>>,
}

#[derive(PartialEq, Debug)]
pub enum Action {
    Task(TargetTask),
    Command(Command),
}
