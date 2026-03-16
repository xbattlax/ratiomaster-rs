/// Proxy abstraction layer supporting SOCKS4, SOCKS4a, SOCKS5, and HTTP CONNECT.
pub mod http;
pub mod socks4;
pub mod socks4a;
pub mod socks5;

use std::net::Ipv4Addr;
use std::time::Duration;

use tokio::net::TcpStream;

use crate::network::tcp;

/// Errors that can occur during proxy operations.
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    /// The proxy rejected the connection request.
    #[error("proxy request rejected: {0}")]
    RequestRejected(String),

    /// Authentication with the proxy failed.
    #[error("proxy authentication failed: {0}")]
    AuthenticationFailed(String),

    /// A protocol-level error occurred.
    #[error("proxy protocol error: {0}")]
    ProtocolError(String),

    /// Failed to connect to the proxy server itself.
    #[error("failed to connect to proxy: {0}")]
    ConnectionFailed(#[from] tcp::TcpError),

    /// An I/O error occurred during proxy communication.
    #[error("proxy io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Proxy configuration.
#[derive(Debug, Clone)]
pub enum ProxyConfig {
    /// Direct connection, no proxy.
    None,

    /// SOCKS4 proxy (requires resolved IPv4 for target).
    Socks4 {
        proxy_host: String,
        proxy_port: u16,
        user_id: String,
    },

    /// SOCKS4a proxy (supports hostname-based targets).
    Socks4a {
        proxy_host: String,
        proxy_port: u16,
        user_id: String,
    },

    /// SOCKS5 proxy (supports IPv4, IPv6, and domain targets).
    Socks5 {
        proxy_host: String,
        proxy_port: u16,
        credentials: Option<socks5::Credentials>,
    },

    /// HTTP CONNECT proxy.
    HttpConnect {
        proxy_host: String,
        proxy_port: u16,
        credentials: Option<http::Credentials>,
    },
}

/// Connects to a target through the configured proxy (or directly if `ProxyConfig::None`).
///
/// Returns a `TcpStream` that is ready for application-level communication.
pub async fn connect(
    config: &ProxyConfig,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
) -> Result<TcpStream, ProxyError> {
    match config {
        ProxyConfig::None => {
            let stream = tcp::connect(target_host, target_port, timeout).await?;
            Ok(stream)
        }

        ProxyConfig::Socks4 {
            proxy_host,
            proxy_port,
            user_id,
        } => {
            let mut stream = tcp::connect(proxy_host, *proxy_port, timeout).await?;
            // SOCKS4 requires a resolved IPv4 address
            let addrs = tcp::resolve(target_host, target_port).await?;
            let ipv4 = addrs
                .iter()
                .find_map(|a| match a.ip() {
                    std::net::IpAddr::V4(v4) => Some(v4),
                    _ => None,
                })
                .ok_or_else(|| {
                    ProxyError::ProtocolError(format!(
                        "SOCKS4 requires IPv4 but no IPv4 address found for {target_host}"
                    ))
                })?;
            socks4::handshake(&mut stream, ipv4, target_port, user_id).await?;
            Ok(stream)
        }

        ProxyConfig::Socks4a {
            proxy_host,
            proxy_port,
            user_id,
        } => {
            let mut stream = tcp::connect(proxy_host, *proxy_port, timeout).await?;
            socks4a::handshake(&mut stream, target_host, target_port, user_id).await?;
            Ok(stream)
        }

        ProxyConfig::Socks5 {
            proxy_host,
            proxy_port,
            credentials,
        } => {
            let mut stream = tcp::connect(proxy_host, *proxy_port, timeout).await?;
            let target_addr = resolve_socks5_address(target_host);
            socks5::handshake(&mut stream, &target_addr, target_port, credentials.as_ref()).await?;
            Ok(stream)
        }

        ProxyConfig::HttpConnect {
            proxy_host,
            proxy_port,
            credentials,
        } => {
            let mut stream = tcp::connect(proxy_host, *proxy_port, timeout).await?;
            http::handshake(&mut stream, target_host, target_port, credentials.as_ref()).await?;
            Ok(stream)
        }
    }
}

/// Resolves a target host string into a SOCKS5 address type.
///
/// If the host parses as an IPv4 or IPv6 address, uses the appropriate type.
/// Otherwise treats it as a domain name.
fn resolve_socks5_address(host: &str) -> socks5::Address {
    if let Ok(v4) = host.parse::<Ipv4Addr>() {
        socks5::Address::Ipv4(v4)
    } else if let Ok(v6) = host.parse::<std::net::Ipv6Addr>() {
        socks5::Address::Ipv6(v6)
    } else {
        socks5::Address::Domain(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_socks5_address_ipv4() {
        match resolve_socks5_address("192.168.1.1") {
            socks5::Address::Ipv4(ip) => assert_eq!(ip, Ipv4Addr::new(192, 168, 1, 1)),
            other => panic!("expected Ipv4, got {other:?}"),
        }
    }

    #[test]
    fn resolve_socks5_address_ipv6() {
        match resolve_socks5_address("::1") {
            socks5::Address::Ipv6(ip) => assert!(ip.is_loopback()),
            other => panic!("expected Ipv6, got {other:?}"),
        }
    }

    #[test]
    fn resolve_socks5_address_domain() {
        match resolve_socks5_address("example.com") {
            socks5::Address::Domain(host) => assert_eq!(host, "example.com"),
            other => panic!("expected Domain, got {other:?}"),
        }
    }

    #[test]
    fn proxy_config_none_is_default() {
        // ProxyConfig::None requires no fields
        let config = ProxyConfig::None;
        assert!(matches!(config, ProxyConfig::None));
    }
}
