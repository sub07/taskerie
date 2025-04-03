mod config;
mod model;
mod service;

use std::path::Path;

pub struct Context {}

pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Context> {
    todo!()
}

#[cfg(test)]
mod test {
    use crate::config::Root;

    use super::*;

    #[test]
    fn playground() {
        const CONF: &str = include_str!("../../taskerie.example.yaml");

        let config: Root = serde_norway::from_str(CONF).unwrap();
        println!("{config:#?}");
    }
}
