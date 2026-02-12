use std::error::Error;
use std::fmt::{
    self,
    Display,
    Formatter,
};
use std::io;

#[derive(Debug)]
pub enum ExecutionError {
    IoError(io::Error),
    ParseTextError(ParseTextError),
    RuntimeError(RuntimeError),
}

impl From<ParseTextError> for ExecutionError {
    fn from(v : ParseTextError) -> Self {
        Self::ParseTextError(v)
    }
}

impl From<RuntimeError> for ExecutionError {
    fn from(v : RuntimeError) -> Self {
        Self::RuntimeError(v)
    }
}

impl From<io::Error> for ExecutionError {
    fn from(v : io::Error) -> Self {
        Self::IoError(v)
    }
}

#[derive(Debug)]
pub struct ParseTextError {
    pub idx : usize,
}

#[derive(Debug, PartialEq, Eq)]
// Token values are 0 indexed
pub enum RuntimeError {
    UnderRead(i64),
    BreforeProgramRead,
    AfterProgramRead,
    TokenLimitHit(i64),
    StackLimitHit(i64),
    NoOut,
    NoTokens,
}

impl Display for RuntimeError {
    fn fmt(&self, f : &mut Formatter<'_>) -> fmt::Result {
        let e = match self {
            Self::UnderRead(t) => {
                format!("Attempted to read from the stack when it is empty, occured at token {t}",)
            },
            Self::BreforeProgramRead => {
                "Moved the execution pointer before the start of the program".to_owned()
            },
            Self::AfterProgramRead => {
                "Moved the execution pointer past the end of the program".to_owned()
            },
            Self::TokenLimitHit(t) => {
                format!("Exceeded the given token limit, occured at token {t}",)
            },
            Self::StackLimitHit(t) => {
                format!("Exceeded the given stack size limit, occured at token {t}",)
            },
            Self::NoOut => "Exited without a value on the stack to return".to_owned(),
            Self::NoTokens => "There are no tokens in the input".to_owned(),
        };
        write!(f, "{e}")
    }
}

impl Error for RuntimeError {}
