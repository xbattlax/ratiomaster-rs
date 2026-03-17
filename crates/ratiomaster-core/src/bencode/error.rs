/// Errors that can occur during BEncode encoding or decoding.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BencodeError {
    /// Unexpected end of input while parsing.
    #[error("unexpected end of input")]
    UnexpectedEof,

    /// An invalid byte was encountered at the given position.
    #[error("invalid byte {byte:#04x} at position {position}")]
    InvalidByte { byte: u8, position: usize },

    /// An integer value was malformed.
    #[error("invalid integer: {0}")]
    InvalidInteger(String),

    /// A string length prefix was malformed or too large.
    #[error("invalid string length: {0}")]
    InvalidStringLength(String),

    /// Trailing data after the decoded value.
    #[error("trailing data: {remaining} bytes after decoded value")]
    TrailingData { remaining: usize },

    /// Dictionary keys were not in sorted order.
    #[error("dictionary keys are not in sorted order")]
    UnsortedKeys,

    /// Nesting depth exceeds the maximum allowed limit.
    #[error("nesting depth exceeds maximum of 128")]
    NestingTooDeep,
}
