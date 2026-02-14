use crate::error::RuntimeError;
use crate::opp::Opp;
use crate::token::Token;

/// A Slug runtime
pub struct Slug {
    pub stack :           Vec<i64>,
    pub stack_limit :     Option<usize>,
    pub tokens :          Vec<Token>,
    /// Pointer to the position in execution
    pub ptr :             i64,
    pub token_limit :     Option<usize>,
    pub tokens_consumed : usize,
    /// Whether or not there is more potential input to be considered
    pub eof :             bool,
}

impl Slug {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            stack :           Vec::new(),
            tokens :          Vec::new(),
            ptr :             0,
            stack_limit :     None,
            token_limit :     None,
            tokens_consumed : 0,
            eof :             false,
        }
    }

    /// Execute a series of inputed tokens.
    ///
    /// # Errors
    ///
    /// See `Self::execute`
    pub fn execute_tokens(&mut self, toks : Vec<Token>) -> Result<Option<i64>, RuntimeError> {
        self.tokens.extend(toks);
        self.execute()
    }

    /// Executes an inputted token.
    ///
    /// # Errors
    ///
    /// See `Self::execute`
    pub fn execute_token(&mut self, token : Token) -> Result<Option<i64>, RuntimeError> {
        self.tokens.push(token);
        self.execute()
    }

    /// Executes the current state of the runtime
    ///
    /// # Errors
    ///
    /// This will error if the runtime enters and invalid state or attempts
    /// an invalid opperation.
    #[expect(clippy::too_many_lines, reason = "Boo Hoo Clippy")]
    pub fn execute(&mut self) -> Result<Option<i64>, RuntimeError> {
        if self.tokens.is_empty() && self.eof {
            return Err(RuntimeError::NoTokens);
        }

        loop {
            if self.ptr < 0 {
                return Err(RuntimeError::BreforeProgramRead);
            }

            #[expect(
                clippy::cast_sign_loss,
                reason = "This function will exit if the pointer is negative"
            )]
            #[expect(
                clippy::cast_possible_truncation,
                reason = "The chances of someone actually writing a program long enough and complex enough to cause a truncation error is so low that I doubt it would ever happen"
            )]
            match self.tokens[self.ptr as usize] {
                Token::Value(i) => self.stack.push(i),

                Token::Opp(opp) => {
                    match opp {
                        Opp::Add => {
                            let rhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            let lhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            self.stack.push(lhs + rhs);
                        },
                        Opp::Sub => {
                            let rhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            let lhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            self.stack.push(lhs - rhs);
                        },
                        Opp::Mul => {
                            let a1 = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            let a2 = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            self.stack.push(a1 * a2);
                        },
                        Opp::Dump => {
                            for (ptr, v) in self.stack.iter().enumerate() {
                                println!("{ptr} | {v}");
                            }
                        },
                        Opp::Top => {
                            let a = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            println!("Top: {a}");
                            self.stack.push(a);
                        },
                        Opp::Swap => {
                            let a1 = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            let a2 = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            self.stack.push(a1);
                            self.stack.push(a2);
                        },
                        Opp::Drop => {
                            self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                        },
                        Opp::Hop => {
                            let d = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            self.ptr += d;
                        },
                        Opp::Div => {
                            let rhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            let lhs = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            self.stack.push(lhs % rhs);
                            self.stack.push(lhs / rhs);
                        },
                        Opp::Pos => {
                            self.stack.push(self.ptr);
                        },
                        Opp::Exit => return self.exit().map(Some),
                        Opp::Goto => {
                            let v = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            self.ptr = v - 1; // Go to the token using 0 index rather than -1 index
                        },
                        Opp::Flip => {
                            let t = self.stack.pop().ok_or(RuntimeError::UnderRead(self.ptr))?;
                            let b = self.stack[0];

                            self.stack[0] = t;
                            self.stack.push(b);
                        },
                    }
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

            #[expect(
                clippy::cast_possible_wrap,
                reason = "The chances of someone writing a program with even over a trillon tokens is so insanely low that this would never happen in a real enviroment"
            )]
            let len = self.tokens.len() as i64;

            if self.ptr == len || self.ptr > len && !self.eof {
                break;
            } else if self.ptr > len && self.eof {
                return Err(RuntimeError::AfterProgramRead);
            }
        }

        if self.eof {
            self.exit().map(Some)
        } else {
            Ok(None)
        }
    }

    /// Exits the program
    ///
    /// # Errors
    /// This will return an error if the stack is empty, otherwise it will
    /// return the topmost value
    pub fn exit(&mut self) -> Result<i64, RuntimeError> {
        self.stack.pop().ok_or(RuntimeError::NoOut)
    }
}

impl Default for Slug {
    fn default() -> Self {
        Self::new()
    }
}
