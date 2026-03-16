/// BEncode codec: decoder, encoder, value types, and error handling.
///
/// BEncoding is the serialization format used by the BitTorrent protocol for
/// .torrent files and tracker communication.
mod decoder;
mod encoder;
mod error;
mod value;

pub use decoder::{decode, decode_prefix};
pub use encoder::{encode, encode_into};
pub use error::BencodeError;
pub use value::BValue;
