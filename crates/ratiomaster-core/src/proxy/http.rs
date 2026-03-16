/// HTTP CONNECT proxy client implementation.
///
/// Sends `CONNECT host:port HTTP/1.1` to the proxy server.
/// Handles 407 Proxy Authentication Required with Basic auth retry.
/// A 2xx response indicates the tunnel is established.
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use super::ProxyError;

/// HTTP CONNECT authentication credentials.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// Performs HTTP CONNECT handshake on an already-connected stream.
///
/// If the proxy responds with 407, retries with Basic authentication
/// if credentials are provided.
pub async fn handshake(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
    credentials: Option<&Credentials>,
) -> Result<(), ProxyError> {
    // First attempt without auth
    let request = build_connect_request(target_host, target_port, None);
    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    let status = read_response_status(stream).await?;

    if (200..300).contains(&status) {
        return Ok(());
    }

    if status == 407 {
        let creds = credentials.ok_or_else(|| {
            ProxyError::AuthenticationFailed(
                "proxy requires authentication (407) but no credentials provided".into(),
            )
        })?;

        // Retry with Basic auth
        let request = build_connect_request(target_host, target_port, Some(creds));
        stream.write_all(request.as_bytes()).await?;
        stream.flush().await?;

        let status = read_response_status(stream).await?;
        if (200..300).contains(&status) {
            return Ok(());
        }

        return Err(ProxyError::AuthenticationFailed(format!(
            "proxy authentication failed with status {status}"
        )));
    }

    Err(ProxyError::RequestRejected(format!(
        "HTTP CONNECT failed with status {status}"
    )))
}

/// Builds the HTTP CONNECT request string.
pub fn build_connect_request(host: &str, port: u16, credentials: Option<&Credentials>) -> String {
    let mut request = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n");

    if let Some(creds) = credentials {
        let encoded = base64_encode(&format!("{}:{}", creds.username, creds.password));
        request.push_str(&format!("Proxy-Authorization: Basic {encoded}\r\n"));
    }

    request.push_str("\r\n");
    request
}

/// Reads the HTTP response status line and consumes headers.
///
/// Returns the status code. Reads until the empty line marking end of headers.
async fn read_response_status(stream: &mut TcpStream) -> Result<u16, ProxyError> {
    let mut reader = BufReader::new(&mut *stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).await?;

    let status = parse_status_line(&status_line)?;

    // Consume remaining headers until empty line
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line == "\r\n" || line == "\n" || line.is_empty() {
            break;
        }
    }

    Ok(status)
}

/// Parses HTTP status code from status line (e.g., "HTTP/1.1 200 OK").
pub fn parse_status_line(line: &str) -> Result<u16, ProxyError> {
    let parts: Vec<&str> = line.trim().splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(ProxyError::ProtocolError(format!(
            "invalid HTTP status line: {line:?}"
        )));
    }
    parts[1]
        .parse::<u16>()
        .map_err(|_| ProxyError::ProtocolError(format!("invalid HTTP status code: {:?}", parts[1])))
}

/// Simple Base64 encoder (no external dependency needed for this).
fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[(triple >> 18) as usize & 0x3F] as char);
        result.push(CHARS[(triple >> 12) as usize & 0x3F] as char);

        if chunk.len() > 1 {
            result.push(CHARS[(triple >> 6) as usize & 0x3F] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[triple as usize & 0x3F] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_no_auth() {
        let req = build_connect_request("example.com", 443, None);
        assert!(req.starts_with("CONNECT example.com:443 HTTP/1.1\r\n"));
        assert!(req.contains("Host: example.com:443\r\n"));
        assert!(req.ends_with("\r\n\r\n"));
        assert!(!req.contains("Proxy-Authorization"));
    }

    #[test]
    fn build_request_with_auth() {
        let creds = Credentials {
            username: "user".into(),
            password: "pass".into(),
        };
        let req = build_connect_request("proxy.test", 8080, Some(&creds));
        assert!(req.contains("Proxy-Authorization: Basic "));
        // user:pass -> dXNlcjpwYXNz
        assert!(req.contains("dXNlcjpwYXNz"));
    }

    #[test]
    fn parse_status_200() {
        assert_eq!(
            parse_status_line("HTTP/1.1 200 Connection established\r\n").unwrap(),
            200
        );
    }

    #[test]
    fn parse_status_407() {
        assert_eq!(
            parse_status_line("HTTP/1.1 407 Proxy Authentication Required\r\n").unwrap(),
            407
        );
    }

    #[test]
    fn parse_status_invalid() {
        assert!(parse_status_line("garbage").is_err());
    }

    #[test]
    fn base64_encode_basic() {
        assert_eq!(base64_encode("user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("a"), "YQ==");
        assert_eq!(base64_encode("ab"), "YWI=");
        assert_eq!(base64_encode("abc"), "YWJj");
    }
}
