use std::{borrow::Cow, str::FromStr};

use itertools::Itertools;

use crate::model::{InterpolatedString, InterpolatedVariable, ParamContext};

impl FromStr for InterpolatedString {
    type Err = anyhow::Error;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        let interpolated_variable_regex = regex::Regex::new(r"\{\{\s*(.*?)\s*\}\}")?;
        let mut acc = 0;

        let parts = interpolated_variable_regex
            .captures_iter(val)
            .map(|captures| {
                let value = captures
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("Could not find captured variable name"))?
                    .as_str();
                let whole = captures
                    .get(0)
                    .ok_or_else(|| anyhow::anyhow!("Could not find whole interpolated variable"))?;
                let start = whole.start() - acc;
                let end = whole.end() - acc;
                acc += whole.len();
                Ok::<_, anyhow::Error>((
                    InterpolatedVariable {
                        name: value.to_string(),
                        start,
                    },
                    end,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut value = val.to_owned();
        for (variable, span_end) in &parts {
            value.replace_range(variable.start..*span_end, "");
        }
        let parts = parts
            .into_iter()
            .map(|(variable, _)| variable)
            .collect_vec();
        Ok(Self { value, parts })
    }
}

impl InterpolatedString {
    pub fn render(&self, param_context: &ParamContext) -> anyhow::Result<Cow<str>> {
        if self.parts.is_empty() {
            Ok(Cow::Borrowed(&self.value))
        } else {
            let mut rendered = self.value.clone();
            let mut acc = 0;
            for part in &self.parts {
                let value = param_context.get(&part.name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Could not find value for param {} during string interpolation",
                        part.name
                    )
                })?;
                rendered.insert_str(part.start + acc, value);
                acc += value.len();
            }
            Ok(Cow::Owned(rendered))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_string() {
        let input = "";
        let expected = InterpolatedString {
            value: String::new(),
            parts: vec![],
        };
        assert_eq!(expected, InterpolatedString::from_str(input).unwrap());
    }

    #[test]
    fn test_single_variable() {
        let input = "{{name}}";
        let expected = InterpolatedString {
            value: String::new(),
            parts: vec![InterpolatedVariable {
                name: "name".to_string(),
                start: 0,
            }],
        };
        assert_eq!(expected, InterpolatedString::from_str(input).unwrap());
    }

    #[test]
    fn test_multiple_variables() {
        let input = "{{name}} is {{age}} years old";
        let expected = InterpolatedString {
            value: " is  years old".to_string(),
            parts: vec![
                InterpolatedVariable {
                    name: "name".to_string(),
                    start: 0,
                },
                InterpolatedVariable {
                    name: "age".to_string(),
                    start: 4,
                },
            ],
        };
        assert_eq!(expected, InterpolatedString::from_str(input).unwrap());
    }

    #[test]
    fn test_multiple_variables_with_inner_spaces() {
        let input = "{{ name }} is {{ age  }} years old";
        let expected = InterpolatedString {
            value: " is  years old".to_string(),
            parts: vec![
                InterpolatedVariable {
                    name: "name".to_string(),
                    start: 0,
                },
                InterpolatedVariable {
                    name: "age".to_string(),
                    start: 4,
                },
            ],
        };
        assert_eq!(expected, InterpolatedString::from_str(input).unwrap());
    }

    #[test]
    fn test_no_variables() {
        let input = "Hello, world!";
        let expected = InterpolatedString {
            value: "Hello, world!".to_string(),
            parts: vec![],
        };
        assert_eq!(expected, InterpolatedString::from_str(input).unwrap());
    }

    #[test]
    fn test_single_variable_with_whitespace() {
        let input = " {{name}} ";
        let expected = InterpolatedString {
            value: "  ".to_string(),
            parts: vec![InterpolatedVariable {
                name: "name".to_string(),
                start: 1,
            }],
        };
        assert_eq!(expected, InterpolatedString::from_str(input).unwrap());
    }

    #[test]
    fn test_render_empty_string() {
        let interpolated = InterpolatedString {
            value: String::new(),
            parts: vec![],
        };
        let context = ParamContext::default();
        assert_eq!(interpolated.render(&context).unwrap(), "");
    }

    #[test]
    fn test_render_no_variables() {
        let interpolated = InterpolatedString {
            value: "Hello, world!".to_string(),
            parts: vec![],
        };
        let context = ParamContext::default();
        assert_eq!(interpolated.render(&context).unwrap(), "Hello, world!");
    }

    #[test]
    fn test_render_single_variable() {
        let interpolated = InterpolatedString {
            value: "Hello, !".to_string(),
            parts: vec![InterpolatedVariable {
                name: "name".to_string(),
                start: 7,
            }],
        };
        let mut context = ParamContext::default();
        context.set("name", "world");
        assert_eq!(interpolated.render(&context).unwrap(), "Hello, world!");
    }

    #[test]
    fn test_render_multiple_variables() {
        let interpolated = InterpolatedString {
            value: " is  years old".to_string(),
            parts: vec![
                InterpolatedVariable {
                    name: "name".to_string(),
                    start: 0,
                },
                InterpolatedVariable {
                    name: "age".to_string(),
                    start: 4,
                },
            ],
        };
        let mut context = ParamContext::default();
        context.set("name", "John");
        context.set("age", "30");
        assert_eq!(
            interpolated.render(&context).unwrap(),
            "John is 30 years old"
        );
    }

    #[test]
    fn test_render_missing_variable() {
        let interpolated = InterpolatedString {
            value: "Hello, !".to_string(),
            parts: vec![InterpolatedVariable {
                name: "name".to_string(),
                start: 7,
            }],
        };
        let context = ParamContext::default();
        assert!(interpolated.render(&context).is_err());
    }
}
