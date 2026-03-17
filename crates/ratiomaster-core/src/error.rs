/// Top-level error type for the ratiomaster-core crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error occurred during BEncode decoding or encoding.
    #[error("bencode error: {0}")]
    Bencode(#[from] crate::bencode::BencodeError),

    /// An error occurred during torrent parsing.
    #[error("torrent error: {0}")]
    Torrent(#[from] crate::torrent::TorrentError),

    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A TCP network error occurred.
    #[error("tcp error: {0}")]
    Tcp(#[from] crate::network::tcp::TcpError),

    /// A proxy error occurred.
    #[error("proxy error: {0}")]
    Proxy(#[from] crate::proxy::ProxyError),

    /// An HTTP error occurred.
    #[error("http error: {0}")]
    Http(#[from] crate::network::http::HttpError),

    /// A tracker response error occurred.
    #[error("tracker response error: {0}")]
    TrackerResponse(#[from] crate::tracker::response::TrackerResponseError),

    /// A scrape error occurred.
    #[error("scrape error: {0}")]
    Scrape(#[from] crate::tracker::scrape::ScrapeError),

    /// An engine error occurred.
    #[error("engine error: {0}")]
    Engine(#[from] crate::engine::EngineError),

    /// A configuration error occurred.
    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),
}

/// Convenience type alias for Results using the crate-level Error.
pub type Result<T> = std::result::Result<T, Error>;
