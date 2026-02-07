use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, Read},
    path::PathBuf,
    str::FromStr,
};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Subcommand,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    /// Run a file.
    Run {
        /// File to take as input to run.
        file: String,
        /// Maximimum number of tokens executed, useful to debug infinite recursion.
        #[arg(short, long)]
        token_limit: Option<usize>,
        /// Maximum size of the stack.
        #[arg(short, long)]
        stack_limit: Option<usize>,
    },
}

fn main() -> io::Result<()> {
    let args = Cli::parse();

    match args.command {
        Subcommand::Run {
            file,
            token_limit,
            stack_limit,
        } => {
            match File::open(&file) {
                Ok(mut data) if PathBuf::from(file).is_file() => {
                    let mut buf = String::new();

                    data.read_to_string(&mut buf)?;

                    let tokens: Vec<Token> = buf
                        .split_ascii_whitespace()
                        .enumerate()
                        .map(|(idx, word)| {
                            word.parse::<Token>().unwrap_or_else(|_| {
                                panic!("Non parseable input file, issue at token {idx}")
                            })
                        })
                        .collect();

                    let output = run(tokens, token_limit, stack_limit);
                    match output {
                        Ok(res) => println!("{res}"),
                        Err(err) => eprintln!("{err}"),
                    }
                }

                Err(err) => eprintln!("Unable to open file with reason: {err}"),

                _ => eprintln!("Is that a file?"),
            };
        }
    }

    Ok(())
}

/// Executes a stream of input tokens.
pub fn run(
    tokens: Vec<Token>,
    token_limit: Option<usize>,
    stack_limit: Option<usize>,
) -> Result<i32, RuntimeError> {
    if tokens.is_empty() {
        return Err(RuntimeError::NoTokens);
    }
    let mut stack: Vec<i32> = Vec::new();

    let mut idx = 0i64;
    let mut tokens_consumed = 0;

    loop {
        if idx < 0 {
            return Err(RuntimeError::BreforeProgramRead);
        }

        match tokens[idx as usize] {
            Token::Num(i) => stack.push(i),

            Token::Opp(opp) => match opp {
                Opp::Add => {
                    let rhs = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    let lhs = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    stack.push(lhs + rhs);
                }

                Opp::Sub => {
                    let rhs = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    let lhs = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    stack.push(lhs - rhs);
                }

                Opp::Mul => {
                    let a1 = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    let a2 = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    stack.push(a1 * a2);
                }

                Opp::Dump => {
                    for (ptr, v) in stack.iter().enumerate() {
                        println!("{ptr} | {v}")
                    }
                }

                Opp::Top => {
                    let a = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    println!("Top: {a}");
                    stack.push(a);
                }

                Opp::Swap => {
                    let a1 = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    let a2 = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    stack.push(a1);
                    stack.push(a2);
                }

                Opp::Drop => {
                    stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                }

                Opp::Hop => {
                    let d = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    idx += d as i64;
                }

                Opp::Div => {
                    let rhs = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    let lhs = stack.pop().ok_or(RuntimeError::UnderRead(idx))?;
                    stack.push(lhs % rhs);
                    stack.push(lhs / rhs);
                }
            },
        }

        idx += 1;

        // Only bother with token limit if it is passed in
        if let Some(limit) = token_limit {
            tokens_consumed += 1;
            if limit < tokens_consumed {
                return Err(RuntimeError::TokenLimitHit);
            }
        }

        if let Some(limit) = stack_limit
            && limit < stack.len()
        {
            return Err(RuntimeError::StackLimitHit);
        }

        if idx == tokens.len() as i64 {
            break;
        } else if idx > tokens.len() as i64 {
            return Err(RuntimeError::AfterProgramRead);
        }
    }

    stack.pop().ok_or(RuntimeError::NoOut)
}

#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeError {
    UnderRead(i64),
    BreforeProgramRead,
    AfterProgramRead,
    TokenLimitHit,
    StackLimitHit,
    NoOut,
    NoTokens,
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let e = match self {
            RuntimeError::UnderRead(t) => {
                format!("Attempted to read from the stack when it is empty, at token {t}")
            }
            RuntimeError::BreforeProgramRead => {
                "Moved the execution pointer before the start of the program".to_owned()
            }
            RuntimeError::AfterProgramRead => {
                "Moved the execution pointer past the end of the program".to_owned()
            }
            RuntimeError::TokenLimitHit => "Exceeded the given token limit".to_owned(),
            RuntimeError::StackLimitHit => "Exceeded the given stack size limit".to_owned(),
            RuntimeError::NoOut => "Exited without a value on the stack to return".to_owned(),
            RuntimeError::NoTokens => "There are no tokens in the input".to_owned(),
        };
        write!(f, "{e}")
    }
}

impl Error for RuntimeError {}

#[derive(Debug, Clone, Copy, Hash)]
pub enum Token {
    Num(i32),
    Opp(Opp),
}

impl FromStr for Token {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(num) = s.parse::<i32>() {
            Ok(Self::Num(num))
        } else if let Ok(op) = s.parse::<Opp>() {
            Ok(Self::Opp(op))
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, Copy, Hash)]
#[repr(u8)]
pub enum Opp {
    Add,
    Sub,
    Mul,
    Dump,
    Top,
    Swap,
    Drop,
    Hop,
    Div,
}

impl FromStr for Opp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(Opp::Add),
            "sub" => Ok(Opp::Sub),
            "mul" => Ok(Opp::Mul),
            "dump" => Ok(Opp::Dump),
            "top" => Ok(Opp::Top),
            "swap" => Ok(Opp::Swap),
            "drop" => Ok(Opp::Drop),
            "hop" => Ok(Opp::Hop),
            "div" => Ok(Opp::Div),
            _ => Err(()),
        }
    }
}

impl Display for Opp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let t = match self {
            Opp::Add => "add",
            Opp::Sub => "sub",
            Opp::Mul => "mul",
            Opp::Dump => "dump",
            Opp::Top => "top",
            Opp::Swap => "swap",
            Opp::Drop => "drop",
            Opp::Hop => "hop",
            Opp::Div => "div",
        };
        write!(f, "{t}")
    }
}
