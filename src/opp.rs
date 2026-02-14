use std::fmt::{
    self,
    Display,
    Formatter,
};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Hash)]
#[repr(u8)]
pub enum Opp {
    Add,
    Sub,
    Mul,
    /// Note: Pushes 2 values, the output and the remainder
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
    /// Exits the program
    Exit,
    Goto,
    Flip,
}

impl FromStr for Opp {
    type Err = ();

    fn from_str(s : &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(Self::Add),
            "sub" => Ok(Self::Sub),
            "mul" => Ok(Self::Mul),
            "dump" => Ok(Self::Dump),
            "top" => Ok(Self::Top),
            "swap" => Ok(Self::Swap),
            "drop" => Ok(Self::Drop),
            "hop" => Ok(Self::Hop),
            "div" => Ok(Self::Div),
            "pos" => Ok(Self::Pos),
            "exit" => Ok(Self::Exit),
            "goto" => Ok(Self::Goto),
            "flip" => Ok(Self::Flip),
            _ => Err(()),
        }
    }
}

impl Display for Opp {
    fn fmt(&self, f : &mut Formatter<'_>) -> fmt::Result {
        let t = match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
            Self::Dump => "dump",
            Self::Top => "top",
            Self::Swap => "swap",
            Self::Drop => "drop",
            Self::Hop => "hop",
            Self::Div => "div",
            Self::Pos => "pos",
            Self::Exit => "exit",
            Self::Goto => "goto",
            Self::Flip => "flip",
        };
        write!(f, "{t}")
    }
}
