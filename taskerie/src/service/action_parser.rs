use std::{
    str::{Chars, FromStr},
    sync::LazyLock,
};

use anyhow::{anyhow, bail, ensure};
use itertools::Itertools;

use crate::model::action::{self, Action};
#[derive(PartialEq, Clone, Copy, Debug)]
enum ReadNameState {
    Target,
    Name,
    Done,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ReadArgState {
    Literal,
    LiteralGroup,
    Interpolated,
    InterpolatedInGroup,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ParserState {
    Start,
    ReadName(ReadNameState),
    ReadArg(ReadArgState),
    Done,
}

struct ActionParser<'a> {
    remainder: Chars<'a>,
    read_count: usize,

    action: action::Action,

    arg_buf: String,
    arg_component_buf: Vec<action::ArgumentComponent>,
    previous_component: Option<action::ArgumentComponent>,

    state: ParserState,
}

impl<'a> ActionParser<'a> {
    const ACTION_NAME_TASK_PREFIX: char = '_';

    fn parse(mut self) -> anyhow::Result<Action> {
        loop {
            match self.state {
                ParserState::Start => self.state = ParserState::ReadName(ReadNameState::Target),
                ParserState::ReadName(state) => self.handle_read_name(state)?,
                ParserState::ReadArg(state) => self.handle_read_arg(state)?,
                ParserState::Done => {
                    ensure!(
                        self.arg_buf.is_empty() && self.arg_component_buf.is_empty(),
                        "Unfinished action"
                    );
                    let remainder = self.remainder.next();
                    ensure!(
                        remainder.is_none(),
                        "Parsing finished with remaining character: {remainder:?}"
                    );
                    return Ok(self.action);
                }
            }
        }
    }

    fn handle_end_literal_component(&mut self, allow_empty: bool) {
        if allow_empty || !self.arg_buf.is_empty() {
            let component = action::ArgumentComponent::Literal(self.arg_buf.clone());
            self.previous_component = Some(component.clone());
            self.arg_component_buf.push(component);
            self.arg_buf.clear();
        }
    }

    fn handle_end_interpolation_component(&mut self) {
        if !self.arg_buf.is_empty() {
            let component = action::ArgumentComponent::Interpolated(self.arg_buf.clone());
            self.previous_component = Some(component.clone());
            self.arg_component_buf.push(component);
            self.arg_buf.clear();
        }
    }

    fn handle_end_arg(&mut self) {
        if !self.arg_component_buf.is_empty() {
            self.previous_component = None;
            self.action.arguments.push(action::Argument {
                components: self.arg_component_buf.clone(),
            });
            self.arg_component_buf.clear();
        }
    }

    fn set_state_reset_arg(&mut self) {
        self.state = ParserState::ReadArg(ReadArgState::Literal);
    }

    fn assert_valid_interpolation_start(&mut self) -> anyhow::Result<()> {
        ensure!(
            self.next_err("$ must be followed by {")? == '{',
            "$ must be followed by {{"
        );
        Ok(())
    }

    fn handle_read_arg(&mut self, read_arg_state: ReadArgState) -> anyhow::Result<()> {
        match read_arg_state {
            ReadArgState::Literal => match self.next() {
                Some('$') => {
                    self.assert_valid_interpolation_start()?;
                    self.handle_end_literal_component(false);
                    self.state = ParserState::ReadArg(ReadArgState::Interpolated);
                }
                Some('"') => self.state = ParserState::ReadArg(ReadArgState::LiteralGroup),
                Some(c) if c.is_whitespace() => {
                    self.handle_end_literal_component(false);
                    self.handle_end_arg();
                }
                Some(c) => {
                    self.arg_buf.push(c);
                }
                None => {
                    self.handle_end_literal_component(false);
                    self.handle_end_arg();
                    self.state = ParserState::Done
                }
            },
            ReadArgState::LiteralGroup => match self.next_err("argument group must be closed")? {
                '"' => {
                    self.handle_end_literal_component(true);
                    self.handle_end_arg();
                    ensure!(
                        self.next().is_none_or(char::is_whitespace),
                        "literal group must be followed by whitespace"
                    );
                    self.set_state_reset_arg();
                }
                '$' => {
                    self.assert_valid_interpolation_start()?;
                    self.handle_end_literal_component(false);
                    self.state = ParserState::ReadArg(ReadArgState::InterpolatedInGroup)
                }
                c => self.arg_buf.push(c),
            },
            ReadArgState::Interpolated => self.handle_interpolated(false)?,
            ReadArgState::InterpolatedInGroup => self.handle_interpolated(true)?,
        }
        Ok(())
    }

    fn handle_interpolated(&mut self, from_group: bool) -> anyhow::Result<()> {
        match self.next_err("interpolation must be closed")? {
            '}' => {
                ensure!(
                    !self.arg_buf.is_empty(),
                    "interpolated value cannot be empty"
                );
                self.handle_end_interpolation_component();
                if from_group {
                    self.state = ParserState::ReadArg(ReadArgState::LiteralGroup);
                } else {
                    self.set_state_reset_arg();
                }
            }
            c => self.arg_buf.push(c),
        }
        Ok(())
    }

    fn handle_read_name(&mut self, read_name_state: ReadNameState) -> anyhow::Result<()> {
        match read_name_state {
            ReadNameState::Target => {
                let first_char = self.next_err("empty action")?;
                if first_char == Self::ACTION_NAME_TASK_PREFIX {
                    self.action.target = action::Target::Task;
                } else {
                    self.action.name.push(first_char);
                }
                self.state = ParserState::ReadName(ReadNameState::Name);
            }
            ReadNameState::Name => match self.next() {
                None => self.state = ParserState::Done,
                Some(c) if c.is_whitespace() => {
                    self.state = ParserState::ReadName(ReadNameState::Done)
                }
                Some(c) => self.action.name.push(c),
            },
            ReadNameState::Done => {
                ensure!(!self.action.name.is_empty(), "Action must have a name");
                self.set_state_reset_arg();
            }
        }
        Ok(())
    }

    fn reset_buf(&mut self) {
        self.arg_buf.clear();
        self.arg_component_buf.clear();
    }

    fn next_err<S: AsRef<str>>(&mut self, err: S) -> anyhow::Result<char> {
        self.next().ok_or(anyhow::anyhow!(
            "Error at {}: {}",
            self.read_count,
            err.as_ref()
        ))
    }

    fn next_non_whitespace(&mut self) -> Option<char> {
        while let Some(c) = self.next() {
            if !c.is_whitespace() {
                return Some(c);
            }
        }
        None
    }

    fn next_non_whitespace_err(&mut self, err: String) -> anyhow::Result<char> {
        self.next_non_whitespace()
            .ok_or(anyhow::anyhow!("Error at {}: {err}", self.read_count))
    }

    fn new<'b: 'a>(value: &'b str) -> ActionParser<'a> {
        eprintln!("Parsing {value}");
        ActionParser {
            remainder: value.chars(),
            read_count: 0,

            action: action::Action {
                name: Default::default(),
                arguments: Default::default(),
                target: action::Target::External,
            },

            arg_buf: Default::default(),
            arg_component_buf: Default::default(),
            previous_component: None,

            state: ParserState::Start,
        }
    }
}

impl Iterator for ActionParser<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_count += 1;
        let c = self.remainder.next();
        eprintln!("Read {c:?} while being in {:?}", self.state);
        c
    }
}

impl FromStr for Action {
    type Err = anyhow::Error;

    fn from_str(mut action_str: &str) -> anyhow::Result<Self> {
        action_str = action_str.trim();
        ensure!(!action_str.is_empty(), "Empty action is invalid");
        let parser = ActionParser::new(action_str);
        parser.parse()
    }
}

#[cfg(test)]
mod test {
    use crate::model::action::{Argument, ArgumentComponent, Target};

    use super::*;

    fn assert_valid_action(
        input: &str,
        expected_action_name: &str,
        expected_arguments: Vec<action::Argument>,
        expected_target: action::Target,
    ) {
        let action = input.parse::<Action>().expect("Expected valid action");
        assert_eq!(expected_action_name, action.name);
        assert_eq!(expected_arguments, action.arguments);
        assert_eq!(expected_target, action.target);
    }

    fn assert_invalid_action<S: AsRef<str>>(input: &str, err: S) {
        let action = input.parse::<Action>();
        let error_str = action.expect_err("Expected invalid action").to_string();
        assert_eq!(err.as_ref(), &error_str);
    }

    #[test]
    fn test_external_program_no_args() {
        assert_valid_action("echo", "echo", vec![], Target::External);
    }

    #[test]
    fn test_external_program_simple_args() {
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
    fn test_external_program_grouped_arg() {
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
    fn test_external_program_simple_grouped_args() {
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
    fn test_external_program_interpolated_arg() {
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
    fn test_external_program_interpolated_args() {
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
    fn test_external_program_mixed_args() {
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
            r#"my_action "" ${arg1} """#,
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
    fn test_empty_interpolation_variable_is_rejected() {
        assert_invalid_action("_my_task ${}test${}", "interpolated value cannot be empty");
    }

    #[test]
    fn test_unclosed_quotation_marks_are_rejected() {
        assert_invalid_action(
            r#"_my_task ""#,
            "Error at 11: argument group must be closed",
        );
    }

    #[test]
    fn test_dollar_sign_must_be_followed_by_curly_brace() {
        assert_invalid_action(
            r#"my_action "hello!@#$%^&*() world""#,
            "$ must be followed by {",
        );
    }

    #[test]
    fn test_dollar_sign_in_task_must_be_followed_by_curly_brace() {
        assert_invalid_action(
            r#"_my_task "hello!@#$%^&*() world""#,
            "$ must be followed by {",
        );
    }

    #[test]
    fn test_empty_interpolation_is_rejected() {
        assert_invalid_action("my_action ${}test${}", "interpolated value cannot be empty");
    }
}
