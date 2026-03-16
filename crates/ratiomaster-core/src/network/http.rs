/// Raw HTTP client for tracker communication.
///
/// Uses a custom implementation (not reqwest) for exact header control,
/// HTTP/1.0 support, and proxy integration. Sends raw HTTP GET requests,
/// reads responses with timeout, and handles gzip + chunked encoding.
use std::io;
use std::time::Duration;

use flate2::read::GzDecoder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::proxy::{self, ProxyConfig};

/// Errors that can occur during HTTP operations.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// Failed to connect (directly or through proxy).
    #[error("connection error: {0}")]
    Connection(String),

    /// The HTTP response was malformed.
    #[error("invalid HTTP response: {0}")]
    InvalidResponse(String),

    /// An I/O error occurred.
    #[error("http io error: {0}")]
    Io(#[from] io::Error),

    /// Request timed out.
    #[error("http request timed out")]
    Timeout,

    /// Proxy error.
    #[error("proxy error: {0}")]
    Proxy(#[from] proxy::ProxyError),

    /// TCP error.
    #[error("tcp error: {0}")]
    Tcp(#[from] crate::network::tcp::TcpError),
}

/// HTTP protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    Http10,
    Http11,
}

impl HttpVersion {
    fn as_str(self) -> &'static str {
        match self {
            HttpVersion::Http10 => "HTTP/1.0",
            HttpVersion::Http11 => "HTTP/1.1",
        }
    }
}

/// A parsed HTTP response.
#[derive(Debug)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status_code: u16,
    /// Response headers (lowercase keys).
    pub headers: Vec<(String, String)>,
    /// Response body (decompressed if gzip).
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Gets the first header value matching the given lowercase key.
    pub fn header(&self, key: &str) -> Option<&str> {
        let key_lower = key.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k == &key_lower)
            .map(|(_, v)| v.as_str())
    }
}

/// Sends an HTTP GET request and returns the response.
///
/// This is a low-level HTTP client designed for BitTorrent tracker communication.
/// It supports:
/// - HTTP/1.0 and HTTP/1.1
/// - Custom headers (for client emulation)
/// - Proxy connections (SOCKS4/4a/5, HTTP CONNECT)
/// - gzip decompression
/// - Chunked transfer encoding
pub async fn get(
    url: &str,
    headers: &[(String, String)],
    version: HttpVersion,
    proxy_config: &ProxyConfig,
    timeout: Duration,
) -> Result<HttpResponse, HttpError> {
    let parsed = parse_url(url)?;

    let stream = proxy::connect(proxy_config, &parsed.host, parsed.port, timeout).await?;

    let request = build_get_request(&parsed, headers, version);

    if parsed.tls {
        let provider = rustls::crypto::ring::default_provider();
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(provider))
            .with_safe_default_protocol_versions()
            .map_err(|e| HttpError::InvalidResponse(format!("TLS config error: {e}")))?
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let server_name = rustls::pki_types::ServerName::try_from(parsed.host.clone())
            .map_err(|e| HttpError::InvalidResponse(format!("invalid TLS server name: {e}")))?;
        let mut tls_stream = connector
            .connect(server_name, stream)
            .await
            .map_err(HttpError::Io)?;

        tls_stream.write_all(request.as_bytes()).await?;
        tls_stream.flush().await?;
        let raw = read_response_data(&mut tls_stream, timeout).await?;
        parse_response(&raw)
    } else {
        let mut stream = stream;
        stream.write_all(request.as_bytes()).await?;
        stream.flush().await?;
        let raw = read_response_data(&mut stream, timeout).await?;
        parse_response(&raw)
    }
}

/// Parsed URL components.
struct ParsedUrl {
    host: String,
    port: u16,
    path_and_query: String,
    tls: bool,
}

fn parse_url(url: &str) -> Result<ParsedUrl, HttpError> {
    let url = url.trim();

    let (without_scheme, tls, default_port) = if let Some(rest) = url.strip_prefix("https://") {
        (rest, true, 443u16)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (rest, false, 80u16)
    } else {
        return Err(HttpError::InvalidResponse(format!(
            "unsupported URL scheme: {url}"
        )));
    };

    let (host_port, path_and_query) = match without_scheme.find('/') {
        Some(i) => (&without_scheme[..i], &without_scheme[i..]),
        None => (without_scheme, "/"),
    };

    let (host, port) = match host_port.rfind(':') {
        Some(i) => {
            let port_str = &host_port[i + 1..];
            let port = port_str
                .parse::<u16>()
                .map_err(|_| HttpError::InvalidResponse(format!("invalid port: {port_str}")))?;
            (&host_port[..i], port)
        }
        None => (host_port, default_port),
    };

    Ok(ParsedUrl {
        host: host.to_string(),
        port,
        path_and_query: path_and_query.to_string(),
        tls,
    })
}

fn build_get_request(
    parsed: &ParsedUrl,
    headers: &[(String, String)],
    version: HttpVersion,
) -> String {
    let mut request = format!(
        "GET {} {}\r\nHost: {}\r\n",
        parsed.path_and_query,
        version.as_str(),
        parsed.host,
    );

    for (key, value) in headers {
        request.push_str(&format!("{key}: {value}\r\n"));
    }

    request.push_str("\r\n");
    request
}

async fn read_response_data<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
    timeout: Duration,
) -> Result<Vec<u8>, HttpError> {
    let mut data = Vec::new();
    let mut buf = [0u8; 32768]; // 32KB chunks, same as original RM

    let result = tokio::time::timeout(timeout, async {
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&buf[..n]),
                Err(e) => return Err(HttpError::Io(e)),
            }
        }
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(data),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            if data.is_empty() {
                Err(HttpError::Timeout)
            } else {
                // Partial data on timeout — return what we have
                Ok(data)
            }
        }
    }
}

/// Parses a raw HTTP response into status, headers, and body.
pub fn parse_response(raw: &[u8]) -> Result<HttpResponse, HttpError> {
    // Find header/body separator
    let header_end = find_header_end(raw)
        .ok_or_else(|| HttpError::InvalidResponse("could not find end of HTTP headers".into()))?;

    let header_bytes = &raw[..header_end];
    let body_start = header_end + 4; // skip \r\n\r\n
    let raw_body = if body_start <= raw.len() {
        &raw[body_start..]
    } else {
        &[]
    };

    let header_str =
        std::str::from_utf8(header_bytes).map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

    let mut lines = header_str.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| HttpError::InvalidResponse("empty response".into()))?;

    let status_code = parse_status_code(status_line)?;

    let mut headers = Vec::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.push((key.trim().to_lowercase(), value.trim().to_string()));
        }
    }

    // Handle chunked transfer encoding
    let is_chunked = headers
        .iter()
        .any(|(k, v)| k == "transfer-encoding" && v.to_lowercase().contains("chunked"));

    let body_data = if is_chunked {
        decode_chunked(raw_body)?
    } else {
        raw_body.to_vec()
    };

    // Handle gzip content encoding
    let is_gzip = headers
        .iter()
        .any(|(k, v)| k == "content-encoding" && v.to_lowercase().contains("gzip"));

    let body = if is_gzip {
        decompress_gzip(&body_data)?
    } else {
        body_data
    };

    Ok(HttpResponse {
        status_code,
        headers,
        body,
    })
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_status_code(status_line: &str) -> Result<u16, HttpError> {
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(HttpError::InvalidResponse(format!(
            "invalid status line: {status_line:?}"
        )));
    }
    parts[1]
        .parse::<u16>()
        .map_err(|_| HttpError::InvalidResponse(format!("invalid status code: {:?}", parts[1])))
}

/// Decodes chunked transfer encoding.
///
/// Format: `<hex-size>\r\n<data>\r\n` repeated, terminated by `0\r\n\r\n`.
pub fn decode_chunked(data: &[u8]) -> Result<Vec<u8>, HttpError> {
    let mut result = Vec::new();
    let mut pos = 0;

    while let Some(i) = data[pos..].windows(2).position(|w| w == b"\r\n") {
        let line_end = pos + i;

        let size_str = std::str::from_utf8(&data[pos..line_end])
            .map_err(|e| HttpError::InvalidResponse(e.to_string()))?
            .trim();

        // Chunk extensions (after ;) are ignored
        let size_hex = size_str.split(';').next().unwrap_or("").trim();
        let chunk_size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| HttpError::InvalidResponse(format!("invalid chunk size: {size_hex:?}")))?;

        if chunk_size == 0 {
            break;
        }

        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + chunk_size;

        if chunk_end > data.len() {
            // Partial chunk — take what we can
            result.extend_from_slice(&data[chunk_start..]);
            break;
        }

        result.extend_from_slice(&data[chunk_start..chunk_end]);
        pos = chunk_end + 2; // skip trailing \r\n

        if pos >= data.len() {
            break;
        }
    }

    Ok(result)
}

/// Decompresses gzip-encoded data.
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, HttpError> {
    use std::io::Read;
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| HttpError::InvalidResponse(format!("gzip decompression failed: {e}")))?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_basic() {
        let parsed = parse_url("http://tracker.example.com:6969/announce?info_hash=abc").unwrap();
        assert_eq!(parsed.host, "tracker.example.com");
        assert_eq!(parsed.port, 6969);
        assert_eq!(parsed.path_and_query, "/announce?info_hash=abc");
    }

    #[test]
    fn parse_url_default_port() {
        let parsed = parse_url("http://example.com/path").unwrap();
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 80);
        assert_eq!(parsed.path_and_query, "/path");
    }

    #[test]
    fn parse_url_no_path() {
        let parsed = parse_url("http://example.com").unwrap();
        assert_eq!(parsed.path_and_query, "/");
    }

    #[test]
    fn parse_url_https() {
        let parsed = parse_url("https://example.com/announce").unwrap();
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 443);
        assert_eq!(parsed.path_and_query, "/announce");
        assert!(parsed.tls);
    }

    #[test]
    fn parse_url_invalid_scheme() {
        assert!(parse_url("ftp://example.com").is_err());
    }

    #[test]
    fn build_get_request_http10() {
        let parsed = ParsedUrl {
            host: "tracker.test".into(),
            port: 80,
            path_and_query: "/announce?x=1".into(),
            tls: false,
        };
        let headers = vec![("User-Agent".into(), "Test/1.0".into())];
        let req = build_get_request(&parsed, &headers, HttpVersion::Http10);

        assert!(req.starts_with("GET /announce?x=1 HTTP/1.0\r\n"));
        assert!(req.contains("Host: tracker.test\r\n"));
        assert!(req.contains("User-Agent: Test/1.0\r\n"));
        assert!(req.ends_with("\r\n\r\n"));
    }

    #[test]
    fn build_get_request_http11() {
        let parsed = ParsedUrl {
            host: "example.com".into(),
            port: 8080,
            path_and_query: "/path".into(),
            tls: false,
        };
        let req = build_get_request(&parsed, &[], HttpVersion::Http11);
        assert!(req.contains("HTTP/1.1"));
    }

    #[test]
    fn parse_response_basic() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello";
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.header("content-type"), Some("text/plain"));
        assert_eq!(resp.body, b"Hello");
    }

    #[test]
    fn parse_response_no_body() {
        let raw = b"HTTP/1.1 204 No Content\r\n\r\n";
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.status_code, 204);
        assert!(resp.body.is_empty());
    }

    #[test]
    fn parse_response_302_redirect() {
        let raw = b"HTTP/1.1 302 Found\r\nLocation: http://other.com/path\r\n\r\n";
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.status_code, 302);
        assert_eq!(resp.header("location"), Some("http://other.com/path"));
    }

    #[test]
    fn decode_chunked_basic() {
        let data = b"5\r\nHello\r\n6\r\n World\r\n0\r\n\r\n";
        let decoded = decode_chunked(data).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn decode_chunked_single() {
        let data = b"d\r\nHello, World!\r\n0\r\n\r\n";
        let decoded = decode_chunked(data).unwrap();
        assert_eq!(decoded, b"Hello, World!");
    }

    #[test]
    fn decode_chunked_with_extension() {
        let data = b"5;ext=val\r\nHello\r\n0\r\n\r\n";
        let decoded = decode_chunked(data).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn decompress_gzip_roundtrip() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let original = b"Hello, gzip world!";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = decompress_gzip(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn parse_response_with_chunked() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nHello\r\n0\r\n\r\n";
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.body, b"Hello");
    }

    #[test]
    fn parse_response_with_gzip() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let body = b"compressed body data";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut raw = b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\n\r\n".to_vec();
        raw.extend_from_slice(&compressed);

        let resp = parse_response(&raw).unwrap();
        assert_eq!(resp.body, body);
    }

    #[test]
    fn header_lookup_case_insensitive() {
        let resp = HttpResponse {
            status_code: 200,
            headers: vec![("content-type".into(), "text/html".into())],
            body: vec![],
        };
        assert_eq!(resp.header("Content-Type"), Some("text/html"));
        assert_eq!(resp.header("content-type"), Some("text/html"));
    }
}
