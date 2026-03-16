/// Client emulation engine: profiles, peer ID generation, and key generation.
///
/// Each `ClientProfile` defines how a specific BitTorrent client version identifies
/// itself to trackers, including peer ID format, HTTP headers, query parameters,
/// and protocol version.
pub mod generator;
pub mod profiles;

/// The family of BitTorrent client being emulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientFamily {
    UTorrent,
    BitComet,
    Vuze,
    Azureus,
    BitTorrent,
    Transmission,
    ABC,
    BitLord,
    BTuga,
    BitTornado,
    Burst,
    BitTyrant,
    BitSpirit,
    KTorrent,
    Deluge,
    GnomeBT,
}

/// How the random portion of the peer ID is generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomType {
    /// Characters from `[a-zA-Z0-9]`.
    Alphanumeric,
    /// Characters from `[0-9]`.
    Numeric,
    /// Raw bytes `0x00..=0xFF`.
    Random,
    /// Characters from `[0-9A-F]`.
    Hex,
}

/// Format for the session key sent to the tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFormat {
    /// Decimal digits of the given length.
    Numeric(usize),
    /// Alphanumeric characters of the given length.
    Alphanumeric(usize),
    /// Hex characters of the given length.
    Hex(usize),
}

/// A complete client emulation profile.
///
/// Defines everything needed to impersonate a specific BitTorrent client version
/// when communicating with a tracker.
#[derive(Debug, Clone)]
pub struct ClientProfile {
    /// Human-readable name (e.g. "uTorrent 3.3.2").
    pub name: String,
    /// The client family.
    pub family: ClientFamily,
    /// Version string.
    pub version: String,
    /// Fixed prefix of the 20-byte peer ID.
    pub peer_id_prefix: Vec<u8>,
    /// How random bytes in the peer ID suffix are generated.
    pub peer_id_random_type: RandomType,
    /// Whether to URL-encode the random portion of the peer ID.
    pub peer_id_url_encode: bool,
    /// Whether URL-encoded hex digits are uppercase.
    pub peer_id_url_encode_uppercase: bool,
    /// Format of the session key.
    pub key_format: KeyFormat,
    /// Whether the key uses uppercase hex/alpha.
    pub key_uppercase: bool,
    /// HTTP protocol version string ("HTTP/1.0" or "HTTP/1.1").
    pub http_protocol: String,
    /// Query template with `{infohash}`, `{peerid}`, etc. placeholders.
    pub query_template: String,
    /// HTTP headers template with `{host}` placeholder.
    pub headers_template: String,
    /// Whether info_hash is encoded with uppercase hex digits.
    pub hash_uppercase: bool,
    /// Default number of peers to request.
    pub default_numwant: u16,
    /// Whether to request compact peer format.
    pub compact: bool,
    /// Whether to request peers without peer IDs.
    pub no_peer_id: bool,
}
