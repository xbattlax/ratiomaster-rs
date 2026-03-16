/// Torrent file (.torrent) parsing and metadata types.
mod error;
mod parser;
mod types;

pub use error::TorrentError;
pub use parser::parse;
pub use types::{TorrentFile, TorrentMetainfo};
