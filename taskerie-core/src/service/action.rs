use indexmap::IndexMap;

use crate::{
    config,
    model::{self},
};

impl TryFrom<config::Action> for model::Action {
    type Error = anyhow::Error;

    fn try_from(action: config::Action) -> Result<Self, Self::Error> {
        Ok(match action {
            config::Action::TaskCall { name, params } => {
                model::Action::TaskCall(model::action::TaskCall {
                    name,
                    params: params
                        .into_iter()
                        .map(|(key, value)| value.parse().map(|value| (key, value)))
                        .collect::<Result<IndexMap<_, _>, _>>()?,
                })
            }
            config::Action::Command(command) => model::Action::Command(command.parse()?),
        })
    }
}
