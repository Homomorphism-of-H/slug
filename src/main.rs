#[cfg(target_os = "linux")]
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, ErrorKind, Read, Write},
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
    /// Formats a file.
    Fmt {
        file: String,
        #[arg(short, long)]
        new_lines: Option<bool>,
        #[arg(long)]
        out: Option<String>,
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
                Ok(mut data) => {
                    println!("Running {file}");
                    let mut buf = String::new();

                    data.read_to_string(&mut buf)?;

                    let tokens = parse_text(buf);

                    let output = run(tokens, token_limit, stack_limit);
                    match output {
                        Ok(res) => println!("{res}"),
                        Err(err) => eprintln!("{err}"),
                    }
                }

                Err(err) => return Err(err),
            };
        }
        Subcommand::Fmt {
            file,
            new_lines,
            out,
        } => match File::options().write(true).read(true).open(&file) {
            Ok(mut data) => {
                println!("Formatting {file}");
                let mut buf = String::new();

                data.read_to_string(&mut buf)?;

                let tokens = parse_text(buf);

                drop(data);

                let mut out = match out {
                    Some(path) => match File::options()
                        .write(true)
                        .read(true)
                        .truncate(true)
                        .open(&path)
                    {
                        Ok(o) => o,
                        Err(err) => {
                            if err.kind() == ErrorKind::NotFound {
                                File::create_new(path)?
                            } else {
                                panic!("Unable to open file with reason: {err}")
                            }
                        }
                    },
                    None => match File::options()
                        .write(true)
                        .read(true)
                        .truncate(true)
                        .open(&file)
                    {
                        Ok(o) => o,
                        Err(err) => panic!("Unable to open file with reason: {err}"),
                    },
                };

                out.lock()?;

                let whitespace = if new_lines.unwrap_or(true) { "\n" } else { " " };

                let mut text = String::new();
                for token in tokens {
                    text += &format!("{token}").to_string();
                    text += whitespace;
                }

                out.write_all(text.as_bytes())?;
            }

            Err(err) => return Err(err),
        },
    }

    Ok(())
}

pub fn parse_text(text: String) -> Vec<Token> {
    text.split_ascii_whitespace()
        .enumerate()
        .map(|(idx, word)| {
            word.parse::<Token>()
                .unwrap_or_else(|_| panic!("Non parseable input file, issue at token {idx}"))
        })
        .collect()
}

#[derive(Debug)]
pub struct ParseTextError {
    pub token_id: usize,
}

/// Executes a stream of input tokens.
pub fn run(
    tokens: Vec<Token>,
    token_limit: Option<usize>,
    stack_limit: Option<usize>,
) -> Result<i64, RuntimeError> {
    if tokens.is_empty() {
        return Err(RuntimeError::NoTokens);
    }
    let mut stack: Vec<i64> = Vec::new();

    let mut ptr = 0i64;
    let mut tokens_consumed = 0;

    loop {
        if ptr < 0 {
            return Err(RuntimeError::BreforeProgramRead);
        }

        match tokens[ptr as usize] {
            Token::Num(i) => stack.push(i),

            Token::Opp(opp) => match opp {
                Opp::Add => {
                    let rhs = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    let lhs = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    stack.push(lhs + rhs);
                }
                Opp::Sub => {
                    let rhs = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    let lhs = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    stack.push(lhs - rhs);
                }
                Opp::Mul => {
                    let a1 = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    let a2 = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    stack.push(a1 * a2);
                }
                Opp::Dump => {
                    for (ptr, v) in stack.iter().enumerate() {
                        println!("{ptr} | {v}")
                    }
                }
                Opp::Top => {
                    let a = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    println!("Top: {a}");
                    stack.push(a);
                }
                Opp::Swap => {
                    let a1 = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    let a2 = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    stack.push(a1);
                    stack.push(a2);
                }
                Opp::Drop => {
                    stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                }
                Opp::Hop => {
                    let d = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    ptr += d;
                }
                Opp::Div => {
                    let rhs = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    let lhs = stack.pop().ok_or(RuntimeError::UnderRead(ptr))?;
                    stack.push(lhs % rhs);
                    stack.push(lhs / rhs);
                }
                Opp::Pos => {
                    stack.push(ptr);
                }
            },
        }

        ptr += 1;

        // Only bother with token limit if it is passed in
        if let Some(limit) = token_limit {
            tokens_consumed += 1;
            if limit < tokens_consumed {
                return Err(RuntimeError::TokenLimitHit(ptr));
            }
        }

        if let Some(limit) = stack_limit
            && limit < stack.len()
        {
            return Err(RuntimeError::StackLimitHit(ptr));
        }

        if ptr == tokens.len() as i64 {
            break;
        } else if ptr > tokens.len() as i64 {
            return Err(RuntimeError::AfterProgramRead);
        }
    }

    stack.pop().ok_or(RuntimeError::NoOut)
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let e = match self {
            RuntimeError::UnderRead(t) => {
                format!("Attempted to read from the stack when it is empty, occured at token {t}",)
            }
            RuntimeError::BreforeProgramRead => {
                "Moved the execution pointer before the start of the program".to_owned()
            }
            RuntimeError::AfterProgramRead => {
                "Moved the execution pointer past the end of the program".to_owned()
            }
            RuntimeError::TokenLimitHit(t) => {
                format!("Exceeded the given token limit, occured at token {t}",)
            }
            RuntimeError::StackLimitHit(t) => {
                format!("Exceeded the given stack size limit, occured at token {t}",)
            }
            RuntimeError::NoOut => "Exited without a value on the stack to return".to_owned(),
            RuntimeError::NoTokens => "There are no tokens in the input".to_owned(),
        };
        write!(f, "{e}")
    }
}

impl Error for RuntimeError {}

#[derive(Debug, Clone, Copy, Hash)]
pub enum Token {
    Num(i64),
    Opp(Opp),
}

impl FromStr for Token {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let t = match self {
            Token::Num(i) => format!("{i}"),
            Token::Opp(i) => format!("{i}"),
        };
        write!(f, "{t}")
    }
}

#[derive(Debug, Clone, Copy, Hash)]
#[repr(u8)]
pub enum Opp {
    Add,
    Sub,
    Mul,
    /// Dump the stack into the output
    Dump,
    /// Prints the topmost value on the stack
    Top,
    /// Swaps the top two values on the stack
    Swap,
    /// Drops the top value from the stack
    Drop,
    Hop,
    Div,
    /// Push the position of the pointer onto the stack
    Pos,
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
            "pos" => Ok(Opp::Pos),
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
            Opp::Pos => "pos",
        };
        write!(f, "{t}")
    }
}
