use std::str::FromStr;

use anyhow::ensure;
use logos::{Lexer, Logos};

use crate::model::action;

mod command;
mod task;

#[derive(Logos, Debug)]
enum ArgumentToken<'a> {
    #[regex(r"[^\{\{]+")]
    Literal(&'a str),
    #[regex(r"\{\{[^{\}\}}]+\}\}", extract_variable_name)]
    Interpolated(&'a str),
}

fn extract_variable_name<'a>(lex: &mut Lexer<'a, ArgumentToken<'a>>) -> &'a str {
    lex.slice()
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim()
}

impl<'a> From<ArgumentToken<'a>> for action::ArgumentPart {
    fn from(value: ArgumentToken<'a>) -> Self {
        match value {
            ArgumentToken::Literal(val) => action::ArgumentPart::Literal(val.to_owned()),
            ArgumentToken::Interpolated(val) => action::ArgumentPart::Variable(val.to_owned()),
        }
    }
}

fn extract_literal<'a, T>(lex: &mut Lexer<'a, T>) -> Option<Vec<ArgumentToken<'a>>>
where
    T: Logos<'a>,
    T::Source: AsRef<str>,
    &'a str: From<<T::Source as logos::Source>::Slice<'a>>,
{
    let arg_lexer = ArgumentToken::lexer(lex.slice().into());
    arg_lexer.collect::<Result<Vec<_>, _>>().ok()
}

fn extract_quoted<'a, T>(lex: &mut Lexer<'a, T>) -> Option<Vec<ArgumentToken<'a>>>
where
    T: Logos<'a>,
    T::Source: AsRef<str>,
    &'a str: From<<T::Source as logos::Source>::Slice<'a>>,
{
    let slice: &str = lex.slice().into();
    let arg_lexer = ArgumentToken::lexer(slice.trim_matches('"'));
    arg_lexer.collect::<Result<Vec<_>, _>>().ok()
}

/// The action name prefix used to identify task actions.
const ACTION_NAME_TASK_PREFIX: char = ':';

impl FromStr for action::Action {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        ensure!(!s.is_empty(), "Action should not be empty");

        let action = if s.starts_with(ACTION_NAME_TASK_PREFIX) {
            action::Action::Task(s[1..].parse()?)
        } else {
            action::Action::Command(s.parse()?)
        };

        Ok(action)
    }
}
