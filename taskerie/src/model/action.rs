#[derive(PartialEq, Debug)]
pub enum Target {
    Task,
    External,
}

#[derive(PartialEq, Debug)]
pub enum ArgumentComponent {
    Literal(String),
    Interpolated(String),
}

#[derive(PartialEq, Debug)]
pub struct Argument {
    pub components: Vec<ArgumentComponent>,
}

#[derive(Debug)]
pub struct Action {
    pub name: String,
    pub arguments: Vec<Argument>,
    pub target: Target,
}
