use crate::{
    config,
    model::{self, task},
};

impl TryFrom<config::Task> for model::task::Task {
    type Error = anyhow::Error;

    fn try_from(value: config::Task) -> Result<Self, Self::Error> {
        Ok(model::task::Task {
            actions: value
                .actions
                .into_iter()
                .map(|action| action.parse())
                .collect::<anyhow::Result<Vec<_>>>()?,
            on_success: value
                .on_success
                .into_iter()
                .map(|action| action.parse())
                .collect::<anyhow::Result<Vec<_>>>()?,
            on_failure: value
                .on_failure
                .into_iter()
                .map(|action| action.parse())
                .collect::<anyhow::Result<Vec<_>>>()?,
            params: value
                .params
                .into_iter()
                .map(|(name, param)| (name, param.into()))
                .collect(),
        })
    }
}

impl From<config::Param> for task::Param {
    fn from(param: config::Param) -> Self {
        task::Param {
            default: param.default,
        }
    }
}
