use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GlasbeyError {
    #[error(
        "invalid hex color length {length}; expected 3 or 6 hex digits with optional leading '#'"
    )]
    InvalidHexLength { length: usize },

    #[error("invalid hex digit '{ch}' at byte {index}; expected ASCII hexadecimal digit")]
    InvalidHexDigit { index: usize, ch: char },
}

pub type Result<T> = std::result::Result<T, GlasbeyError>;
