#[derive(Debug)]
pub enum Target {
    Task,
    Program,
}

#[derive(PartialEq, Debug)]
pub enum Argument {
    Literal(String),
    Interpolated(String),
}

pub struct Action {
    pub name: String,
    pub arguments: Vec<Argument>,
    pub target: Target,
}
