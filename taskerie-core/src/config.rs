use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Root {
    pub tasks: IndexMap<String, Task>,
}

#[derive(Debug)]
pub enum Action {
    TaskCall {
        name: String,
        params: IndexMap<String, String>,
    },
    Command(String),
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ActionVisitor;

        impl<'de> serde::de::Visitor<'de> for ActionVisitor {
            type Value = Action;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A task reference or a command")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Action::Command(v.to_owned()))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let (task_name, params) = map
                    .next_entry::<String, IndexMap<String, String>>()?
                    .ok_or(serde::de::Error::custom("Unexpected empty task"))?;

                if let Ok(Some(_)) = map.next_key::<String>() {
                    return Err(serde::de::Error::custom("Unexpected extra key"));
                }

                Ok(Action::TaskCall {
                    name: task_name,
                    params,
                })
            }
        }

        deserializer.deserialize_any(ActionVisitor)
    }
}

#[derive(Deserialize, Debug)]
pub struct Task {
    pub actions: Vec<Action>,
    #[serde(default)]
    pub on_failure: Vec<Action>,
    #[serde(default)]
    pub on_success: Vec<Action>,
    #[serde(default)]
    pub params: IndexMap<String, Param>,
}

#[derive(Deserialize, Debug)]
pub struct Param {
    pub default: Option<String>,
}
