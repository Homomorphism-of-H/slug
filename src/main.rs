use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, Read},
    path::PathBuf,
    str::FromStr,
};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None, name = "ass")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Subcommand,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    Run {
        #[arg(short, long)]
        file: String,
    },
}

fn main() -> io::Result<()> {
    let args = Cli::parse();

    match args.command {
        Subcommand::Run { file } => {
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

                    println!("{:?}", run(tokens));
                }

                Err(err) => eprintln!("Unable to open file due to with reason: {err}"),

                _ => eprintln!("Is that a file?"),
            };
        }
    }

    Ok(())
}

pub fn run(tokens: impl IntoIterator<Item = Token>) -> Result<i32, RuntimeError> {
    let mut stack: Vec<i32> = Vec::new();

    let mut hop_dist: i32 = 0;

    for (idx, token) in tokens.into_iter().enumerate() {
        if hop_dist <= 0 {
            hop_dist = 0;
            match token {
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
                        hop_dist += d;
                    }
                },
            }
        } else {
            hop_dist -= 1;
        }
    }

    stack.pop().ok_or(RuntimeError::NoOut)
}

#[derive(Debug)]
pub enum RuntimeError {
    StackOverfill,
    UnderRead(usize),
    PrematureEndOfProgram,
    NoOut,
}

#[derive(Debug)]
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
        };
        write!(f, "{t}")
    }
}
