/// Errors that can occur during torrent file parsing.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TorrentError {
    /// A required field was missing from the torrent metadata.
    #[error("missing required field: {0}")]
    MissingField(String),

    /// The torrent structure was invalid.
    #[error("invalid torrent structure: {0}")]
    InvalidStructure(String),

    /// An error occurred during BEncode decoding.
    #[error("bencode decode error: {0}")]
    Bencode(String),
}
