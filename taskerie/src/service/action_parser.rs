use std::{str::FromStr, sync::LazyLock};

use anyhow::{anyhow, bail, ensure};
use logos::{Lexer, Logos};
use regex::Regex;

use crate::model::action::{self, Action, Argument, Target};

#[derive(Logos, PartialEq, Debug)]
#[logos(skip r"[\s\t]+")]
enum ArgumentsContext {
    #[regex(r#"[^\s"]+"#, parse_word_arg_context)]
    Word(Vec<ArgumentContext>),
    #[regex(r#""[^"]*""#, parse_word_group_arg_context)]
    WordGroup(Vec<ArgumentContext>),
}

fn parse_word_arg_context(lexer: &mut Lexer<ArgumentsContext>) -> Option<Vec<ArgumentContext>> {
    dbg!(lexer.slice());
    let arg_lexer = ArgumentContext::lexer(lexer.slice());
    arg_lexer.collect::<Result<Vec<_>, _>>().ok()
}

fn parse_word_group_arg_context(
    lexer: &mut Lexer<ArgumentsContext>,
) -> Option<Vec<ArgumentContext>> {
    dbg!(lexer.slice().trim());
    let arg_lexer = ArgumentContext::lexer(lexer.slice().trim_matches('"'));
    arg_lexer.collect::<Result<Vec<_>, _>>().ok()
}

#[derive(Logos, PartialEq, Clone, Debug)]
enum ArgumentContext {
    #[regex("[^$]+", |lex| lex.slice().to_string())]
    Literal(String),
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

impl FromStr for Action {
    type Err = anyhow::Error;

    fn from_str(action_str: &str) -> anyhow::Result<Self> {
        ensure!(!action_str.is_empty(), "Empty action is invalid");

        // Action name parsing
        let action_name_match = ACTION_NAME_REGEX
            .find(&action_str)
            .ok_or(anyhow!("Invalid action name"))?;

        let action_name = action_name_match.as_str();
        let action_str = &action_str[action_name_match.end()..];

        let (action_name, action_target) =
            if let Some(underscore_prefixed_action_name) = action_name.strip_prefix("_") {
                (underscore_prefixed_action_name, action::Target::Task)
            } else {
                (action_name, action::Target::External)
            };

        // Arguments parsing
        let mut args_lex = ArgumentsContext::lexer(action_str);
        let mut action_arguments = Vec::new();
        while let Some(Ok(token)) = args_lex.next() {
            let arg_tokens = match token {
                ArgumentsContext::Word(args) => args,
                ArgumentsContext::WordGroup(args) => args,
            };
            action_arguments.push(action::Argument {
                components: arg_tokens
                    .into_iter()
                    .map(|token| match token {
                        ArgumentContext::Literal(val) => action::ArgumentComponent::Literal(val),
                        ArgumentContext::Interpolated(val) => {
                            action::ArgumentComponent::Interpolated(val)
                        }
                    })
                    .collect(),
            });
        }

        Ok(Action {
            name: action_name.into(),
            arguments: action_arguments,
            target: action_target,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::model::action::ArgumentComponent;

    use super::*;

    fn assert_valid_action(
        input: &str,
        expected_action_name: &str,
        expected_arguments: Vec<action::Argument>,
        expected_target: action::Target,
    ) {
        let action = input.parse::<Action>().expect("Valid action");
        assert_eq!(expected_action_name, action.name);
        assert_eq!(expected_arguments, action.arguments);
        assert_eq!(expected_target, action.target);
    }

    #[test]
    fn test_program_no_args() {
        assert_valid_action("echo", "echo", vec![], Target::External);
    }

    #[test]
    fn test_program_simple_args() {
        assert_valid_action(
            "echo hello world",
            "echo",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Literal("hello".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("world".into())],
                },
            ],
            Target::External,
        );
    }

    #[test]
    fn test_program_grouped_arg() {
        assert_valid_action(
            r#"echo "hello world""#,
            "echo",
            vec![Argument {
                components: vec![ArgumentComponent::Literal("hello world".into())],
            }],
            Target::External,
        );
    }

    #[test]
    fn test_program_simple_grouped_args() {
        assert_valid_action(
            r#"echo "hello world" ! good "hello again" yoohoo"#,
            "echo",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Literal("hello world".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("!".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("good".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("hello again".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("yoohoo".into())],
                },
            ],
            Target::External,
        );
    }

    #[test]
    fn test_program_interpolated_arg() {
        assert_valid_action(
            "echo ${arg1}",
            "echo",
            vec![Argument {
                components: vec![ArgumentComponent::Interpolated("arg1".into())],
            }],
            Target::External,
        );
    }

    #[test]
    fn test_program_interpolated_args() {
        assert_valid_action(
            "echo ${arg1} ${arg2}",
            "echo",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg1".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg2".into())],
                },
            ],
            Target::External,
        );
    }

    #[test]
    fn test_program_mixed_args() {
        assert_valid_action(
            r#"echo-test ${arg1} hello "are you there" ${arg2} mmh "boo" ${arg3}"#,
            "echo-test",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg1".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("hello".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("are you there".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg2".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("mmh".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("boo".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg3".into())],
                },
            ],
            Target::External,
        );
    }

    #[test]
    fn test_task_no_args() {
        assert_valid_action("_my_task", "my_task", vec![], Target::Task);
    }

    #[test]
    fn test_task_simple_args() {
        assert_valid_action(
            "_my_task hello world",
            "my_task",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Literal("hello".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("world".into())],
                },
            ],
            Target::Task,
        );
    }

    #[test]
    fn test_task_grouped_arg() {
        assert_valid_action(
            r#"_my_task "hello world""#,
            "my_task",
            vec![Argument {
                components: vec![ArgumentComponent::Literal("hello world".into())],
            }],
            Target::Task,
        );
    }

    #[test]
    fn test_task_simple_grouped_args() {
        assert_valid_action(
            r#"_my_task "hello world" ! good "hello again" yoohoo"#,
            "my_task",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Literal("hello world".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("!".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("good".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("hello again".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("yoohoo".into())],
                },
            ],
            Target::Task,
        );
    }

    #[test]
    fn test_task_interpolated_arg() {
        assert_valid_action(
            "_my_task ${arg1}",
            "my_task",
            vec![Argument {
                components: vec![ArgumentComponent::Interpolated("arg1".into())],
            }],
            Target::Task,
        );
    }

    #[test]
    fn test_task_interpolated_args() {
        assert_valid_action(
            "_my_task ${arg1} ${arg2}",
            "my_task",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg1".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg2".into())],
                },
            ],
            Target::Task,
        );
    }

    #[test]
    fn test_task_mixed_args() {
        assert_valid_action(
            r#"_$ezgrmy_task-with-a$complx^name ${arg1} hello "are you there" ${arg2} mmh "boo" ${arg3}"#,
            "$ezgrmy_task-with-a$complx^name",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg1".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("hello".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("are you there".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg2".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("mmh".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("boo".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg3".into())],
                },
            ],
            Target::Task,
        );
    }

    #[test]
    fn test_single_argument_with_multiple_components() {
        assert_valid_action(
            "my_action ${arg1}test${arg2}",
            "my_action",
            vec![Argument {
                components: vec![
                    ArgumentComponent::Interpolated("arg1".into()),
                    ArgumentComponent::Literal("test".into()),
                    ArgumentComponent::Interpolated("arg2".into()),
                ],
            }],
            Target::External,
        );
    }

    #[test]
    fn test_multiple_arguments_with_multiple_components() {
        assert_valid_action(
            r#"my_action ${arg1}test${arg2} "hello world" ${arg3}literal${arg4}"#,
            "my_action",
            vec![
                Argument {
                    components: vec![
                        ArgumentComponent::Interpolated("arg1".into()),
                        ArgumentComponent::Literal("test".into()),
                        ArgumentComponent::Interpolated("arg2".into()),
                    ],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("hello world".into())],
                },
                Argument {
                    components: vec![
                        ArgumentComponent::Interpolated("arg3".into()),
                        ArgumentComponent::Literal("literal".into()),
                        ArgumentComponent::Interpolated("arg4".into()),
                    ],
                },
            ],
            Target::External,
        );
    }

    #[test]
    fn test_edge_case_empty_literal() {
        assert_valid_action(
            r#"my_action ""${arg1}""#,
            "my_action",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Literal("".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg1".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("".into())],
                },
            ],
            Target::External,
        );
    }

    #[test]
    fn test_complex_mixed_arguments() {
        assert_valid_action(
            r#"my_action ${arg1}test${arg2} "hello world" ${arg3}literal${arg4} "another literal" ${arg5}"#,
            "my_action",
            vec![
                Argument {
                    components: vec![
                        ArgumentComponent::Interpolated("arg1".into()),
                        ArgumentComponent::Literal("test".into()),
                        ArgumentComponent::Interpolated("arg2".into()),
                    ],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("hello world".into())],
                },
                Argument {
                    components: vec![
                        ArgumentComponent::Interpolated("arg3".into()),
                        ArgumentComponent::Literal("literal".into()),
                        ArgumentComponent::Interpolated("arg4".into()),
                    ],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("another literal".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg5".into())],
                },
            ],
            Target::External,
        );
    }

    #[test]
    fn test_task_with_multiple_components() {
        assert_valid_action(
            r#"_my_task ${arg1}test${arg2} "hello world" ${arg3}literal${arg4}"#,
            "my_task",
            vec![
                Argument {
                    components: vec![
                        ArgumentComponent::Interpolated("arg1".into()),
                        ArgumentComponent::Literal("test".into()),
                        ArgumentComponent::Interpolated("arg2".into()),
                    ],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("hello world".into())],
                },
                Argument {
                    components: vec![
                        ArgumentComponent::Interpolated("arg3".into()),
                        ArgumentComponent::Literal("literal".into()),
                        ArgumentComponent::Interpolated("arg4".into()),
                    ],
                },
            ],
            Target::Task,
        );
    }

    #[test]
    fn test_task_with_edge_case_empty_interpolation() {
        assert_valid_action(
            "_my_task ${}test${}",
            "my_task",
            vec![Argument {
                components: vec![
                    ArgumentComponent::Interpolated("".into()),
                    ArgumentComponent::Literal("test".into()),
                    ArgumentComponent::Interpolated("".into()),
                ],
            }],
            Target::Task,
        );
    }

    #[test]
    fn test_task_with_edge_case_empty_literal() {
        assert_valid_action(
            r#"_my_task ""${arg1}""#,
            "my_task",
            vec![
                Argument {
                    components: vec![ArgumentComponent::Literal("".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg1".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("".into())],
                },
            ],
            Target::Task,
        );
    }

    #[test]
    fn test_task_with_complex_mixed_arguments() {
        assert_valid_action(
            r#"_my_task ${arg1}test${arg2} "hello world" ${arg3}literal${arg4} "another literal" ${arg5}"#,
            "my_task",
            vec![
                Argument {
                    components: vec![
                        ArgumentComponent::Interpolated("arg1".into()),
                        ArgumentComponent::Literal("test".into()),
                        ArgumentComponent::Interpolated("arg2".into()),
                    ],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("hello world".into())],
                },
                Argument {
                    components: vec![
                        ArgumentComponent::Interpolated("arg3".into()),
                        ArgumentComponent::Literal("literal".into()),
                        ArgumentComponent::Interpolated("arg4".into()),
                    ],
                },
                Argument {
                    components: vec![ArgumentComponent::Literal("another literal".into())],
                },
                Argument {
                    components: vec![ArgumentComponent::Interpolated("arg5".into())],
                },
            ],
            Target::Task,
        );
    }

    #[test]
    fn test_grouped_argument_with_interpolation() {
        assert_valid_action(
            r#"my_action "hello ${arg1} world""#,
            "my_action",
            vec![Argument {
                components: vec![
                    ArgumentComponent::Literal("hello ".into()),
                    ArgumentComponent::Interpolated("arg1".into()),
                    ArgumentComponent::Literal(" world".into()),
                ],
            }],
            Target::External,
        );
    }

    #[test]
    fn test_grouped_argument_with_special_characters() {
        assert_valid_action(
            r#"my_action "hello!@#$%^&*() world""#,
            "my_action",
            vec![Argument {
                components: vec![ArgumentComponent::Literal("hello!@#$%^&*() world".into())],
            }],
            Target::External,
        );
    }

    #[test]
    fn test_task_with_grouped_argument_with_special_characters() {
        assert_valid_action(
            r#"_my_task "hello!@#$%^&*() world""#,
            "my_task",
            vec![Argument {
                components: vec![ArgumentComponent::Literal("hello!@#$%^&*() world".into())],
            }],
            Target::Task,
        );
    }

    #[test]
    fn test_task_with_grouped_argument_and_interpolation() {
        assert_valid_action(
            r#"_my_task "hello ${arg1} world""#,
            "my_task",
            vec![Argument {
                components: vec![
                    ArgumentComponent::Literal("hello ".into()),
                    ArgumentComponent::Interpolated("arg1".into()),
                    ArgumentComponent::Literal(" world".into()),
                ],
            }],
            Target::Task,
        );
    }

    #[test]
    fn test_edge_case_empty_interpolation_fails() {
        assert_valid_action(
            "my_action ${}test${}",
            "my_action",
            vec![Argument {
                components: vec![
                    ArgumentComponent::Interpolated("".into()),
                    ArgumentComponent::Literal("test".into()),
                    ArgumentComponent::Interpolated("".into()),
                ],
            }],
            Target::External,
        );
    }
}
