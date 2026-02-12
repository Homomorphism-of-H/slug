use std::fs::File;
use std::io::{
    self,
    BufRead,
    ErrorKind,
    Read,
    Write,
    stdin,
};

use clap::Parser;

use crate::error::ExecutionError;
use crate::runtime::Slug;
use crate::token::Tokenizer;

pub mod error;
pub mod opp;
pub mod runtime;
pub mod token;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command : Subcommand,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    /// Run a file.
    Run {
        /// File to take as input to run.
        file :        String,
        /// Maximimum number of tokens executed, useful to debug infinite
        /// recursion.
        #[arg(short, long)]
        token_limit : Option<usize>,
        /// Maximum size of the stack.
        #[arg(short, long)]
        stack_limit : Option<usize>,
    },
    /// Formats a file.
    Fmt {
        /// File to format
        file :      String,
        /// Whether to pad with newlines or spaces
        #[arg(short, long)]
        new_lines : Option<bool>,
        /// Output file of formatting, defaults to the input file.
        #[arg(long)]
        out :       Option<String>,
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
            match run_file(&file, token_limit, stack_limit) {
                Ok(out) => println!("Result: {out}"),
                Err(err) => eprintln!("{err:?}"),
            }
        },
        Subcommand::Fmt {
            file,
            new_lines,
            out,
        } => {
            println!("Formatting {file}");

            format_file(&file, new_lines, out)?;
        },
        Subcommand::Repl => {
            let mut input = stdin().lock();
            let mut runtime = Slug::new();
            loop {
                let mut buf = String::new();
                let out = match input.read_line(&mut buf) {
                    Ok(0) => {
                        let toks = Tokenizer::parse_text(&buf).expect("Unable to parse text");
                        runtime.eof = true;
                        runtime
                            .execute_tokens(toks)
                            .expect("Error during execution")
                    },
                    Ok(_) => {
                        let toks = Tokenizer::parse_text(&buf).expect("Unable to parse text");
                        runtime
                            .execute_tokens(toks)
                            .expect("Error during execution")
                    },
                    Err(e) => return Err(e),
                };

                if let Some(val) = out {
                    println!("{val}");
                    break;
                }
            }
        },
    }

    Ok(())
}

/// Formats a file with optional parameters
///
/// # Errors
/// Errors if the inputed file path can't be read.
///
/// # Panics
/// Panics if the inputed file can't be parsed.
pub fn format_file(file : &str, new_lines : Option<bool>, out : Option<String>) -> io::Result<()> {
    match File::options().write(true).read(true).open(file) {
        Ok(mut data) => {
            let mut buf = String::new();

            data.read_to_string(&mut buf)?;

            let tokens = Tokenizer::parse_text(&buf).expect("Unable to parse text");

            drop(data);

            let mut out = match out {
                Some(path) => {
                    match File::options()
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
                                return Err(err);
                            }
                        },
                    }
                },
                None => {
                    File::options()
                        .write(true)
                        .read(true)
                        .truncate(true)
                        .open(file)?
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
        },

        Err(err) => return Err(err),
    }
    Ok(())
}

/// Runs a file with some optional parameters
///
/// # Errors
/// This function will error if the file can't be opened and read, if the file
/// is syntaxtically invalid, or if the runtime errors during execution of the
/// file
pub fn run_file(
    file : &str,
    token_limit : Option<usize>,
    stack_limit : Option<usize>,
) -> Result<i64, ExecutionError> {
    match File::open(file) {
        Ok(mut data) => {
            println!("Running {file}");
            let mut buf = String::new();
            data.read_to_string(&mut buf)?;

            let tokens = Tokenizer::parse_text(&buf)?;

            let mut runtime = Slug {
                stack : Vec::new(),
                stack_limit,
                tokens,
                ptr : 0,
                token_limit,
                tokens_consumed : 0,
                eof : true,
            };

            let output = runtime.execute();

            match output {
                Ok(Some(res)) => Ok(res),
                Err(err) => Err(ExecutionError::RuntimeError(err)),
                _ => unreachable!(),
            }
        },

        Err(err) => Err(ExecutionError::IoError(err)),
    }
}
