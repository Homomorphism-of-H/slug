use std::fmt::{
    self,
    Display,
    Formatter,
};
use std::str::FromStr;

use crate::error::ParseTextError;
use crate::opp::Opp;

#[derive(Debug, Clone, Copy, Hash)]
pub enum Token {
    Num(i64),
    Opp(Opp),
}

impl FromStr for Token {
    type Err = ();

    fn from_str(s : &str) -> Result<Self, Self::Err> {
        #[expect(
            clippy::option_if_let_else,
            reason = "Clippy's 'solution' is much less readable"
        )]
        if let Ok(num) = s.parse::<i64>() {
            Ok(Self::Num(num))
        } else if let Ok(op) = s.parse::<Opp>() {
            Ok(Self::Opp(op))
        } else {
            Err(())
        }
    }
}

impl Display for Token {
    fn fmt(&self, f : &mut Formatter<'_>) -> fmt::Result {
        let t = match self {
            Self::Num(i) => format!("{i}"),
            Self::Opp(i) => format!("{i}"),
        };
        write!(f, "{t}")
    }
}

pub struct Tokenizer {}

impl Tokenizer {
    /// Tokenizes a string slice
    ///
    /// # Errors
    /// If the inputed text is syntaxtically invalid
    ///
    /// # Panics
    ///
    /// This shouldn't be possible
    pub fn parse_text(text : &str) -> Result<Vec<Token>, ParseTextError> {
        let tokens : Vec<(usize, Result<Token, ()>)> = text
            .split_ascii_whitespace()
            .enumerate()
            .map(|(idx, word)| (idx, word.parse::<Token>()))
            .collect();

        if tokens.iter().all(|(_, tok)| Result::is_ok(tok)) {
            Ok(tokens
                .iter()
                .map(|(_, tok)| tok.expect("Can't happen, the vec has been validated already"))
                .collect())
        } else if let Some((idx, _)) = tokens.iter().find(|(_, tok)| tok.is_err()) {
            Err(ParseTextError {
                idx : *idx
            })
        } else {
            unreachable!();
        }
    }
}
