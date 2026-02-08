use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, BufRead, ErrorKind, Read, Write, stdin},
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
        /// File to format
        file: String,
        #[arg(short, long)]
        new_lines: Option<bool>,
        /// Output file of formatting, defaults to the input file.
        #[arg(long)]
        out: Option<String>,
    },
    /// Creates a Repl to test out the syntax and the control flow.
    Repl,
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

                    let tokens = Tokenizer::parse_text(buf).unwrap();

                    let mut runtime = Slug {
                        stack: Vec::new(),
                        stack_limit,
                        tokens,
                        ptr: 0,
                        token_limit,
                        tokens_consumed: 0,
                        eof: true,
                    };

                    let output = runtime.execute();

                    match output {
                        Ok(Some(res)) => println!("{res}"),
                        Err(err) => eprintln!("Error: {err}"),
                        _ => unreachable!(),
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

                let tokens = Tokenizer::parse_text(buf).unwrap();

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
        Subcommand::Repl => {
            let mut input = stdin().lock();
            let mut runtime = Slug::new();
            loop {
                let mut buf = String::new();
                let out = match input.read_line(&mut buf) {
                    Ok(0) => {
                        let toks = Tokenizer::parse_text(buf).unwrap();
                        runtime.eof = true;
                        runtime.execute_tokens(toks).unwrap()
                    }
                    Ok(_) => {
                        let toks = Tokenizer::parse_text(buf).unwrap();
                        runtime.execute_tokens(toks).unwrap()
                    }
                    Err(e) => return Err(e),
                };
                if let Some(val) = out {
                    println!("{val}");
                    break;
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct ParseTextError {
    pub idx: usize,
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
    Div,
    /// Dump the stack into the output
    Dump,
    /// Prints the topmost value on the stack
    Top,
    /// Swaps the top two values on the stack
    Swap,
    /// Drops the top value from the stack
    Drop,
    /// Hops some amount of tokens fowards or backwards
    Hop,
    /// Push the position of the pointer onto the stack
    Pos,
    /// Exits the program.
    Exit,
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
            "exit" => Ok(Opp::Exit),
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
            Opp::Exit => "exit",
        };
        write!(f, "{t}")
    }
}

pub struct Tokenizer {}

impl Tokenizer {
    /// Tokenizes some text
    pub fn parse_text(text: String) -> Result<Vec<Token>, ParseTextError> {
        let tokens: Vec<(usize, Result<Token, ()>)> = text
            .split_ascii_whitespace()
            .enumerate()
            .map(|(idx, word)| (idx, word.parse::<Token>()))
            .collect();

        if tokens.iter().all(|(_, tok)| Result::is_ok(tok)) {
            Ok(tokens.iter().map(|(_, tok)| tok.unwrap()).collect())
        } else {
            if let Some((idx, _)) = tokens.iter().find(|(_, tok)| tok.is_err()) {
                Err(ParseTextError { idx: *idx })
            } else {
                unreachable!();
            }
        }
    }
}

/// A Slug runtime
pub struct Slug {
    pub stack: Vec<i64>,
    pub stack_limit: Option<usize>,
    pub tokens: Vec<Token>,
    /// Pointer to position in execution
    pub ptr: i64,
    pub token_limit: Option<usize>,
    pub tokens_consumed: usize,
    /// Whether or not there is more potential input to be considered
    pub eof: bool,
}

impl Slug {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            tokens: Vec::new(),
            ptr: 0,
            stack_limit: None,
            token_limit: None,
            tokens_consumed: 0,
            eof: false,
        }
    }

    pub fn execute_tokens(&mut self, toks: Vec<Token>) -> Result<Option<i64>, RuntimeError> {
        self.tokens.extend(toks);
        self.execute()
    }

    pub fn execute_token(&mut self, token: Token) -> Result<Option<i64>, RuntimeError> {
        self.tokens.push(token);
        self.execute()
    }

    pub fn execute(&mut self) -> Result<Option<i64>, RuntimeError> {
        if self.tokens.is_empty() && self.eof {
            return Err(RuntimeError::NoTokens);
        }

        loop {
            if self.ptr < 0 {
                return Err(RuntimeError::BreforeProgramRead);
            }

            match self.tokens[self.ptr as usize] {
                Token::Num(i) => self.stack.push(i),

                Token::Opp(opp) => match opp {
                    Opp::Add => {
                        let rhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        let lhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        self.stack.push(lhs + rhs);
                    }
                    Opp::Sub => {
                        let rhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        let lhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        self.stack.push(lhs - rhs);
                    }
                    Opp::Mul => {
                        let a1 = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        let a2 = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        self.stack.push(a1 * a2);
                    }
                    Opp::Dump => {
                        for (ptr, v) in self.stack.iter().enumerate() {
                            println!("{ptr} | {v}")
                        }
                    }
                    Opp::Top => {
                        let a = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        println!("Top: {a}");
                        self.stack.push(a);
                    }
                    Opp::Swap => {
                        let a1 = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        let a2 = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        self.stack.push(a1);
                        self.stack.push(a2);
                    }
                    Opp::Drop => {
                        self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                    }
                    Opp::Hop => {
                        let d = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        self.ptr += d;
                    }
                    Opp::Div => {
                        let rhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        let lhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        self.stack.push(lhs % rhs);
                        self.stack.push(lhs / rhs);
                    }
                    Opp::Pos => {
                        self.stack.push(self.ptr);
                    }
                    Opp::Exit => return self.exit().map(Some),
                },
            }

            self.ptr += 1;
            self.tokens_consumed += 1;

            // Only bother with token limit if it exists
            if let Some(limit) = self.token_limit
                && limit < self.tokens_consumed
            {
                return Err(RuntimeError::TokenLimitHit(self.ptr));
            }

            if let Some(limit) = self.stack_limit
                && limit < self.stack.len()
            {
                return Err(RuntimeError::StackLimitHit(self.ptr));
            }

            if self.ptr == self.tokens.len() as i64
                || self.ptr > self.tokens.len() as i64 && !self.eof
            {
                break;
            } else if self.ptr > self.tokens.len() as i64 && self.eof {
                return Err(RuntimeError::AfterProgramRead);
            }
        }

        if self.eof {
            self.exit().map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn exit(&mut self) -> Result<i64, RuntimeError> {
        self.stack.pop().ok_or(RuntimeError::NoOut)
    }
}

impl Default for Slug {
    fn default() -> Self {
        Self::new()
    }
}
