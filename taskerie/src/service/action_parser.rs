use anyhow::{bail, ensure};
use logos::Logos;

use crate::model::action::{Action, Argument, Target};

#[derive(Logos, PartialEq)]
#[logos(skip r"[\s\t]+")]
enum ActionToken {
    #[regex(r"[^ ]+", priority = 3)]
    Word,
    #[regex(r#""[^"]*""#)]
    WordGroup,
    #[regex(r"\$\{[^ ]+}")]
    InterpolatedWord,
}

impl TryFrom<String> for Action {
    type Error = anyhow::Error;

    fn try_from(action: String) -> anyhow::Result<Self> {
        let mut lexer = ActionToken::lexer(&action);
        ensure!(
            matches!(lexer.next(), Some(Ok(ActionToken::Word))),
            "The first word must be a command or a task name"
        ); // Maybe allow interpolated value for the action name
        let action_name = lexer.slice();
        let (action_name, action_target) =
            if let Some(stripped_action_name) = action_name.strip_prefix("_") {
                // Find another syntax for task invocation (it's ugly)
                (stripped_action_name, Target::Task)
            } else {
                (action_name, Target::Program)
            };

        let mut arguments = Vec::new();

        while let Some(token) = lexer.next() {
            match token {
                Ok(token) => {
                    let argument = match token {
                        ActionToken::Word => Argument::Literal(lexer.slice().to_owned()),
                        ActionToken::WordGroup => {
                            Argument::Literal(lexer.slice().trim_matches('"').to_owned())
                        }
                        ActionToken::InterpolatedWord => {
                            let token_str = lexer.slice();
                            let word_end = token_str.len() - 1;
                            Argument::Interpolated(token_str[2..word_end].to_owned())
                        }
                    };
                    arguments.push(argument);
                }
                Err(_) => {
                    bail!("Syntax error near col {}", lexer.span().end)
                }
            }
        }

        Ok(Action {
            name: action_name.to_owned(),
            arguments,
            target: action_target,
        })
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
}
