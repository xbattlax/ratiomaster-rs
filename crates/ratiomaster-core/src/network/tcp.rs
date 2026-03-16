/// Async TCP socket layer with configurable timeouts and DNS resolution.
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::TcpStream;

/// Errors that can occur during TCP operations.
#[derive(Debug, thiserror::Error)]
pub enum TcpError {
    /// Connection attempt timed out.
    #[error("connection timed out after {0:?}")]
    Timeout(Duration),

    /// Connection was refused by the remote host.
    #[error("connection refused: {0}")]
    ConnectionRefused(String),

    /// DNS resolution failed.
    #[error("DNS resolution failed for {host}: {source}")]
    DnsFailure { host: String, source: io::Error },

    /// A general I/O error occurred.
    #[error("tcp io error: {0}")]
    Io(#[from] io::Error),
}

/// Resolves a hostname and port to socket addresses.
pub async fn resolve(host: &str, port: u16) -> Result<Vec<SocketAddr>, TcpError> {
    let addr_str = format!("{host}:{port}");
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host(&addr_str)
        .await
        .map_err(|e| TcpError::DnsFailure {
            host: host.to_string(),
            source: e,
        })?
        .collect();

    if addrs.is_empty() {
        return Err(TcpError::DnsFailure {
            host: host.to_string(),
            source: io::Error::new(io::ErrorKind::NotFound, "no addresses found"),
        });
    }

    Ok(addrs)
}

/// Connects to a remote host with a configurable timeout.
///
/// Resolves the hostname, then attempts to connect to the first resolved address.
pub async fn connect(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, TcpError> {
    let addrs = resolve(host, port).await?;

    let fut = TcpStream::connect(addrs.as_slice());
    let stream = tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| TcpError::Timeout(timeout))?
        .map_err(|e| {
            if e.kind() == io::ErrorKind::ConnectionRefused {
                TcpError::ConnectionRefused(format!("{host}:{port}"))
            } else {
                TcpError::Io(e)
            }
        })?;

    Ok(stream)
}

/// Wraps a `TcpStream` in a buffered stream for efficient reads and writes.
pub fn buffered(stream: TcpStream) -> BufStream<TcpStream> {
    BufStream::new(stream)
}

/// Reads all available data from a stream until EOF, with a timeout.
///
/// Reads in chunks of `buf_size` bytes. Returns accumulated data.
pub async fn read_all(
    stream: &mut TcpStream,
    timeout: Duration,
    buf_size: usize,
) -> Result<Vec<u8>, TcpError> {
    let mut data = Vec::new();
    let mut buf = vec![0u8; buf_size];

    let result = tokio::time::timeout(timeout, async {
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&buf[..n]),
                Err(e) => return Err(TcpError::Io(e)),
            }
        }
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(data),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // Timeout — return whatever we got
            if data.is_empty() {
                Err(TcpError::Timeout(timeout))
            } else {
                Ok(data)
            }
        }
    }
}

/// Writes all data to a stream, flushing afterward.
pub async fn write_all(stream: &mut TcpStream, data: &[u8]) -> Result<(), TcpError> {
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolve_localhost() {
        let addrs = resolve("localhost", 80).await.unwrap();
        assert!(!addrs.is_empty());
    }

    #[tokio::test]
    async fn resolve_invalid_host_fails() {
        let result = resolve("this.host.does.not.exist.invalid", 80).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TcpError::DnsFailure { host, .. } => {
                assert_eq!(host, "this.host.does.not.exist.invalid");
            }
            other => panic!("expected DnsFailure, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn connect_timeout_on_unreachable() {
        // 192.0.2.1 is TEST-NET-1 (RFC 5737) — should be unreachable/timeout
        let result = connect("192.0.2.1", 12345, Duration::from_millis(100)).await;
        assert!(result.is_err());
    }

    #[test]
    fn buffered_wraps_stream() {
        // Just a type-level check — BufStream creation is infallible
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let stream = TcpStream::connect(addr).await.unwrap();
            let _buf = buffered(stream);
        });
    }
}
