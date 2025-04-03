use std::str::FromStr;

use anyhow::{bail, ensure};
use indexmap::IndexMap;
use itertools::Itertools;
use logos::Logos;

use crate::{
    model::action,
    service::action_parser::{ACTION_NAME_TASK_PREFIX, extract_literal, extract_quoted},
};

use super::ArgumentToken;

#[derive(Debug, Logos)]
#[logos(skip r"[\s\t]+")]
enum TaskToken<'a> {
    #[regex(r"--[\S]+", |lex| lex.slice().trim_start_matches("--"))]
    Param(&'a str),
    #[regex(r#"(?:\{\{[^\}\}]+\}\}|[^\s\{\{"]+)+"#, extract_literal)]
    Literal(Vec<ArgumentToken<'a>>),
    #[regex(r#""[^"]*""#, extract_quoted)]
    Quoted(Vec<ArgumentToken<'a>>),
}

impl FromStr for action::TargetTask {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        ensure!(
            !s.is_empty(),
            "Expected task name after {}",
            ACTION_NAME_TASK_PREFIX
        );

        let mut lexer = TaskToken::lexer(s);

        // TODO: Dedup with command parsing
        // Start dedup
        let Some(Ok(TaskToken::Literal(task_name_parts))) = lexer.next() else {
            bail!("Task name should be a literal") // TODO: Better error messages for all failing cases
        };

        ensure!(
            task_name_parts.len() == 1,
            "Task name should be a single literal"
        );
        let ArgumentToken::Literal(task_name) = task_name_parts[0] else {
            bail!("Using a variable in a task name is not supported")
        };
        // End dedup

        let mut task = action::TargetTask {
            name: task_name.to_owned(),
            params: IndexMap::new(),
        };

        while let Some(Ok(token)) = lexer.next() {
            if let TaskToken::Param(param) = token {
                match lexer.next() {
                    Some(Ok(TaskToken::Literal(args_token)))
                    | Some(Ok(TaskToken::Quoted(args_token))) => {
                        task.params.insert(
                            param.to_owned(),
                            args_token.into_iter().map_into().collect(),
                        );
                    }
                    Some(Err(_)) => bail!("Could not find value after `{param}`"),
                    None => bail!("Expected value for parameter `{param}`"),
                    _ => bail!("Expected param value after param name `{param}`"),
                }
            } else {
                bail!("Expected param name, got {token:?}")
            }
        }

        Ok(task)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! task_params {
            () => {
                indexmap::IndexMap::new()
            };
            ($($key:literal => $value:expr),* $(,)?) => {
                indexmap::IndexMap::from([
                    $(
                        ($key.to_owned(), $value),
                    )*
                ])
            };
        }

    #[test]
    fn test_valid_task_parsing() {
        let task = "task".parse::<action::TargetTask>().unwrap();
        assert_eq!(task.name, "task");
        assert!(task.params.is_empty());
    }

    #[test]
    fn test_task_with_literal_params() {
        let task = "task --param1 value1 --param2 value2"
            .parse::<action::TargetTask>()
            .unwrap();
        assert_eq!(task.name, "task");
        assert_eq!(
            task_params!(
                "param1" => vec![action::ArgumentPart::Literal("value1".to_owned())],
                "param2" => vec![action::ArgumentPart::Literal("value2".to_owned())]
            ),
            task.params
        );
    }

    #[test]
    fn test_task_with_quoted_params() {
        let task = r#"task --param1 "value1" --param2 "value2 with spaces""#
            .parse::<action::TargetTask>()
            .unwrap();
        assert_eq!(task.name, "task");
        assert_eq!(
            task_params!(
                "param1" => vec![action::ArgumentPart::Literal("value1".to_owned())],
                "param2" => vec![action::ArgumentPart::Literal("value2 with spaces".to_owned())]
            ),
            task.params
        );
    }

    #[test]
    fn test_task_with_simple_interpolation() {
        let task = "task --param1 {{ param1 }} --param2 {{param2}}"
            .parse::<action::TargetTask>()
            .unwrap();
        assert_eq!(task.name, "task");
        assert_eq!(
            task_params!(
                "param1" => vec![action::ArgumentPart::Variable("param1".to_owned())],
                "param2" => vec![action::ArgumentPart::Variable("param2".to_owned())]
            ),
            task.params
        );
    }

    #[test]
    fn test_task_with_quoted_interpolation() {
        let task = r#"task --param1 "{{ param1 }}" --param2 "{{param2}}""#
            .parse::<action::TargetTask>()
            .unwrap();
        assert_eq!(task.name, "task");
        assert_eq!(
            task_params!(
                "param1" => vec![action::ArgumentPart::Variable("param1".to_owned())],
                "param2" => vec![action::ArgumentPart::Variable("param2".to_owned())]
            ),
            task.params
        );
    }

    #[test]
    fn test_task_with_complex_interpolation() {
        let task = r#"task --param1 before{{ param1 }}after --param2 "{{param2}} with spaces""#
            .parse::<action::TargetTask>()
            .unwrap();
        assert_eq!(task.name, "task");
        assert_eq!(
            task_params!(
                "param1" => vec![
                    action::ArgumentPart::Literal("before".to_owned()),
                    action::ArgumentPart::Variable("param1".to_owned()),
                    action::ArgumentPart::Literal("after".to_owned()),
                ],
                "param2" => vec![
                    action::ArgumentPart::Variable("param2".to_owned()),
                    action::ArgumentPart::Literal(" with spaces".to_owned()),
                ]
            ),
            task.params
        );
    }

    #[test]
    fn test_task_fail_with_bad_quoting() {
        let task = r#"task --param1 "{{ param1 }}"#.parse::<action::TargetTask>().unwrap_err();
        assert_eq!("Could not find value after `param1`", task.to_string());
    }
}
