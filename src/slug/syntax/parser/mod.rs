pub mod error;

pub use self::error::*;

pub type ParserResult<T> = Result<T, ParserError>;

pub use super::lexer;
