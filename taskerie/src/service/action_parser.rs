use std::sync::LazyLock;

use anyhow::{anyhow, bail, ensure};
use logos::{Lexer, Logos};
use regex::Regex;

use crate::model::action::{self, Action, Argument, Target};

#[derive(Logos, PartialEq, Debug)]
#[logos(skip r"[\s\t]+")]
enum ArgumentsContext {
    #[regex(r"[^\s]+")]
    Word,
    #[regex(r#""[^"]*""#)]
    WordGroup,
}

#[derive(Logos, PartialEq, Clone)]
enum ArgumentContext {
    #[regex(".+")]
    Literal,
    #[token("${", eval_interpolation)]
    Interpolated(String),
}

fn eval_interpolation(lexer: &mut Lexer<ArgumentContext>) -> Option<String> {
    let mut interpolated_lexer = lexer.clone().morph::<InterpolationContext>();
    if let Some(Ok(InterpolationContext::Word)) = interpolated_lexer.next() {
        let var = interpolated_lexer.slice().to_owned();
        if let Some(Ok(InterpolationContext::End)) = interpolated_lexer.next() {
            *lexer = interpolated_lexer.morph();
            return Some(var);
        }
    }
    None
}

#[derive(Logos, PartialEq)]
enum InterpolationContext {
    #[regex(r"[^\s}]+")]
    Word,
    #[token("}")]
    End,
}

static ACTION_NAME_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[^\s]+").unwrap());

impl TryFrom<String> for Action {
    type Error = anyhow::Error;

    fn try_from(action_str: String) -> anyhow::Result<Self> {
        ensure!(!action_str.is_empty(), "Empty action is invalid");

        // Action name parsing
        let action_name_match = ACTION_NAME_REGEX
            .find(&action_str)
            .ok_or(anyhow!("Invalid action name"))?;

        let action_name = action_name_match.as_str();
        let action_str = &action_str[action_name_match.end()..];

        let (action_name, action_type) =
            if let Some(underscore_prefixed_action_name) = action_name.strip_prefix("_") {
                (underscore_prefixed_action_name, action::Target::Task)
            } else {
                (action_name, action::Target::Program)
            };

        // Arguments parsing
        let mut args_lex = ArgumentsContext::lexer(action_str);

        while let Some(Ok(token)) = args_lex.next() {
            dbg!((token, args_lex.slice()));
        }
        todo!()
    }
}

#[cfg(test)]
mod test {
    use assert_matches::assert_matches;

    use super::*;

    #[test]
    fn test_program_no_args() {
        let input = "echo".to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("echo", action.name);
        assert!(action.arguments.is_empty());
        assert_matches!(action.target, Target::Program);
    }

    #[test]
    fn test_program_simple_args() {
        let input = "echo hello world".to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("echo", action.name);
        assert_eq!(
            vec![
                Argument::Literal("hello".into()),
                Argument::Literal("world".into()),
            ],
            action.arguments
        );
        assert_matches!(action.target, Target::Program);
    }

    #[test]
    fn test_program_grouped_arg() {
        let input = r#"echo "hello world""#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("echo", action.name);
        assert_eq!(
            vec![Argument::Literal("hello world".into()),],
            action.arguments
        );
        assert_matches!(action.target, Target::Program);
    }

    #[test]
    fn test_program_simple_grouped_args() {
        let input = r#"echo "hello world" ! good "hello again" yoohoo"#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("echo", action.name);
        assert_eq!(
            vec![
                Argument::Literal("hello world".into()),
                Argument::Literal("!".into()),
                Argument::Literal("good".into()),
                Argument::Literal("hello again".into()),
                Argument::Literal("yoohoo".into()),
            ],
            action.arguments
        );
        assert_matches!(action.target, Target::Program);
    }

    #[test]
    fn test_program_interpolated_arg() {
        let input = r#"echo ${arg1}"#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("echo", action.name);
        assert_eq!(
            vec![Argument::Interpolated("arg1".into()),],
            action.arguments
        );
        assert_matches!(action.target, Target::Program);
    }

    #[test]
    fn test_program_interpolated_args() {
        let input = r#"echo ${arg1} ${arg2}"#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("echo", action.name);
        assert_eq!(
            vec![
                Argument::Interpolated("arg1".into()),
                Argument::Interpolated("arg2".into()),
            ],
            action.arguments
        );
        assert_matches!(action.target, Target::Program);
    }

    #[test]
    fn test_program_mixed_args() {
        let input = r#"echo ${arg1} hello "are you there" ${arg2} mmh "boo" ${arg3}"#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("echo", action.name);
        assert_eq!(
            vec![
                Argument::Interpolated("arg1".into()),
                Argument::Literal("hello".into()),
                Argument::Literal("are you there".into()),
                Argument::Interpolated("arg2".into()),
                Argument::Literal("mmh".into()),
                Argument::Literal("boo".into()),
                Argument::Interpolated("arg3".into()),
            ],
            action.arguments
        );
        assert_matches!(action.target, Target::Program);
    }

    #[test]
    fn test_task_no_args() {
        let input = "_my_task".to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("my_task", action.name);
        assert!(action.arguments.is_empty());
        assert_matches!(action.target, Target::Task);
    }

    #[test]
    fn test_task_simple_args() {
        let input = "_my_task hello world".to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("my_task", action.name);
        assert_eq!(
            vec![
                Argument::Literal("hello".into()),
                Argument::Literal("world".into()),
            ],
            action.arguments
        );
        assert_matches!(action.target, Target::Task);
    }

    #[test]
    fn test_task_grouped_arg() {
        let input = r#"_my_task "hello world""#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("my_task", action.name);
        assert_eq!(
            vec![Argument::Literal("hello world".into()),],
            action.arguments
        );
        assert_matches!(action.target, Target::Task);
    }

    #[test]
    fn test_task_simple_grouped_args() {
        let input = r#"_my_task "hello world" ! good "hello again" yoohoo"#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("my_task", action.name);
        assert_eq!(
            vec![
                Argument::Literal("hello world".into()),
                Argument::Literal("!".into()),
                Argument::Literal("good".into()),
                Argument::Literal("hello again".into()),
                Argument::Literal("yoohoo".into()),
            ],
            action.arguments
        );
        assert_matches!(action.target, Target::Task);
    }

    #[test]
    fn test_task_interpolated_arg() {
        let input = r#"_my_task ${arg1}"#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("my_task", action.name);
        assert_eq!(
            vec![Argument::Interpolated("arg1".into()),],
            action.arguments
        );
        assert_matches!(action.target, Target::Task);
    }

    #[test]
    fn test_task_interpolated_args() {
        let input = r#"_my_task ${arg1} ${arg2}"#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("my_task", action.name);
        assert_eq!(
            vec![
                Argument::Interpolated("arg1".into()),
                Argument::Interpolated("arg2".into()),
            ],
            action.arguments
        );
        assert_matches!(action.target, Target::Task);
    }

    #[test]
    fn test_task_mixed_args() {
        let input =
            r#"_my_task ${arg1} hello "are you there" ${arg2} mmh "boo" ${arg3}"#.to_string();
        let action = Action::try_from(input).unwrap();
        assert_eq!("my_task", action.name);
        assert_eq!(
            vec![
                Argument::Interpolated("arg1".into()),
                Argument::Literal("hello".into()),
                Argument::Literal("are you there".into()),
                Argument::Interpolated("arg2".into()),
                Argument::Literal("mmh".into()),
                Argument::Literal("boo".into()),
                Argument::Interpolated("arg3".into()),
            ],
            action.arguments
        );
        assert_matches!(action.target, Target::Task);
    }

    #[test]
    fn test_empty_action() {
        let input = "".to_string();
        let action = Action::try_from(input);
        assert_eq!("Expected action name", &action.unwrap_err().to_string())
    }
}
