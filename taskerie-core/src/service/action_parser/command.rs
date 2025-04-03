use std::str::FromStr;

use anyhow::{bail, ensure};
use itertools::Itertools;
use logos::Logos;

use crate::{
    model::action,
    service::action_parser::{extract_literal, extract_quoted},
};

use super::ArgumentToken;

#[derive(Logos, Debug)]
#[logos(skip r"[\s\t]+")]
enum CommandContext<'a> {
    #[regex(r#"(?:\{\{[^\}\}]+\}\}|[^\s\{\{]+)+"#, extract_literal)]
    Literal(Vec<ArgumentToken<'a>>),
    #[regex(r#""[^"]*""#, extract_quoted)]
    Quoted(Vec<ArgumentToken<'a>>),
}

impl FromStr for action::Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        ensure!(!s.is_empty(), "Command should not be empty");
        let mut lexer = CommandContext::lexer(s);
        let Some(Ok(CommandContext::Literal(command_name_parts))) = lexer.next() else {
            bail!("Command name should be a literal") // TODO: Better error messages for all failing cases
        };

        ensure!(
            command_name_parts.len() == 1,
            "Command name should be a single literal"
        );
        let ArgumentToken::Literal(command_name) = command_name_parts[0] else {
            bail!("Using a variable in a command name is not supported")
        };

        let mut command = action::Command {
            name: command_name.to_owned(),
            arguments: Vec::new(),
        };

        while let Some(token) = lexer.next() {
            match token {
                Ok(CommandContext::Literal(parts)) | Ok(CommandContext::Quoted(parts)) => {
                    command
                        .arguments
                        .push(parts.into_iter().map_into().collect_vec());
                }
                Err(_) => bail!(
                    "Could not match any token, remaining string: {}",
                    lexer.remainder()
                ),
            }
        }

        Ok(command)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_command_parsing() {
        let command = "command".parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert!(command.arguments.is_empty());
    }

    #[test]
    fn test_command_parsing_with_arguments() {
        let command = "command arg1 arg2".parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![action::ArgumentPart::Literal("arg1".to_owned())],
                vec![action::ArgumentPart::Literal("arg2".to_owned())],
            ],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_nested_arguments() {
        let command = r#"command arg1 "arg2 arg3""#.parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![action::ArgumentPart::Literal("arg1".to_owned())],
                vec![action::ArgumentPart::Literal("arg2 arg3".to_owned()),],
            ],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_multiple_nested_arguments() {
        let command = r#"command arg1 "arg2 arg3" "arg4 arg5""#.parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![action::ArgumentPart::Literal("arg1".to_owned())],
                vec![action::ArgumentPart::Literal("arg2 arg3".to_owned()),],
                vec![action::ArgumentPart::Literal("arg4 arg5".to_owned()),],
            ],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_multiple_nested_arguments_with_spaces() {
        let command =
            r#"command arg1 "arg2 arg3" "arg4 arg5" arg6"#.parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![action::ArgumentPart::Literal("arg1".to_owned())],
                vec![action::ArgumentPart::Literal("arg2 arg3".to_owned()),],
                vec![action::ArgumentPart::Literal("arg4 arg5".to_owned()),],
                vec![action::ArgumentPart::Literal("arg6".to_owned()),],
            ],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_interpolation() {
        let command = r#"command {{var1}}"#.parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![vec![action::ArgumentPart::Variable("var1".to_owned())],],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_multiple_interpolations() {
        let command = r#"command {{ var1 }} {{ var2 }}"#.parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![action::ArgumentPart::Variable("var1".to_owned())],
                vec![action::ArgumentPart::Variable("var2".to_owned())],
            ],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_interpolations_and_literals() {
        let command = r#"command {{ var1 }} arg2 {{ var3 }}"#.parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![action::ArgumentPart::Variable("var1".to_owned())],
                vec![action::ArgumentPart::Literal("arg2".to_owned())],
                vec![action::ArgumentPart::Variable("var3".to_owned())],
            ],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_interpolations_and_literals_mixed() {
        let command = r#"command {{ var1 }}arg1 "arg2 " "{{ var3 }}" "{{var4}}arg4""#
            .parse::<action::Command>()
            .unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![
                    action::ArgumentPart::Variable("var1".to_owned()),
                    action::ArgumentPart::Literal("arg1".to_owned())
                ],
                vec![action::ArgumentPart::Literal("arg2 ".to_owned()),],
                vec![action::ArgumentPart::Variable("var3".to_owned()),],
                vec![
                    action::ArgumentPart::Variable("var4".to_owned()),
                    action::ArgumentPart::Literal("arg4".to_owned())
                ],
            ],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_interpolations_and_literals_and_spaces_and_trailing_spaces() {
        let command =
            r#"command {{ var1 }} arg2 {{ var3 }} arg4  "#.parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![action::ArgumentPart::Variable("var1".to_owned())],
                vec![action::ArgumentPart::Literal("arg2".to_owned())],
                vec![action::ArgumentPart::Variable("var3".to_owned())],
                vec![action::ArgumentPart::Literal("arg4".to_owned())],
            ],
            command.arguments
        );
    }

    #[test]
    fn test_command_parsing_with_interpolations_and_literals_and_spaces_and_trailing_spaces_and_leading_spaces()
     {
        let command =
            r#"  command {{ var1 }} arg2 {{ var3 }} arg4  "#.parse::<action::Command>().unwrap();
        assert_eq!(command.name, "command");
        assert_eq!(
            vec![
                vec![action::ArgumentPart::Variable("var1".to_owned())],
                vec![action::ArgumentPart::Literal("arg2".to_owned())],
                vec![action::ArgumentPart::Variable("var3".to_owned())],
                vec![action::ArgumentPart::Literal("arg4".to_owned())],
            ],
            command.arguments
        );
    }
}
