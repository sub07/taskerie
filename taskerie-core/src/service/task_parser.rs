use std::convert::TryInto;

use crate::{
    config,
    model::{self, task},
};

impl TryFrom<config::Task> for model::task::Task {
    type Error = anyhow::Error;

    fn try_from(value: config::Task) -> Result<Self, Self::Error> {
        Ok(Self {
            actions: value
                .actions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<anyhow::Result<Vec<_>>>()?,
            on_success: value
                .on_success
                .into_iter()
                .map(TryInto::try_into)
                .collect::<anyhow::Result<Vec<_>>>()?,
            on_failure: value
                .on_failure
                .into_iter()
                .map(TryInto::try_into)
                .collect::<anyhow::Result<Vec<_>>>()?,
            params: value
                .params
                .into_iter()
                .map(|(name, param)| (name, param.into()))
                .collect(),
            working_directory: value.working_directory.map(|dir| dir.parse()).transpose()?,
        })
    }
}

impl From<config::Param> for task::Param {
    fn from(param: config::Param) -> Self {
        Self {
            default: param.default,
        }
    }
}
