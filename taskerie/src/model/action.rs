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

#[derive(Debug)]
pub struct Action {
    pub name: String,
    pub arguments: Vec<Argument>,
    pub target: Target,
}
