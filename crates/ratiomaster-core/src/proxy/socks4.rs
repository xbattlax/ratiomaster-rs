/// SOCKS4 proxy client implementation.
///
/// Protocol:
/// ```text
/// Client -> Proxy:
///   VER(0x04) | CMD(0x01=CONNECT) | DSTPORT(2 bytes) | DSTIP(4 bytes) | USERID | NULL(0x00)
///
/// Proxy -> Client:
///   NULL(0x00) | STATUS | DSTPORT(2 bytes) | DSTIP(4 bytes)
///   STATUS 0x5A = request granted
/// ```
use std::net::Ipv4Addr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::ProxyError;

/// Performs SOCKS4 handshake on an already-connected stream.
///
/// The `target_ip` must be a resolved IPv4 address (SOCKS4 does not support hostnames).
pub async fn handshake(
    stream: &mut TcpStream,
    target_ip: Ipv4Addr,
    target_port: u16,
    user_id: &str,
) -> Result<(), ProxyError> {
    let request = build_request(target_ip, target_port, user_id);
    stream.write_all(&request).await?;
    stream.flush().await?;

    let mut response = [0u8; 8];
    stream.read_exact(&mut response).await?;

    parse_response(&response)
}

/// Builds the SOCKS4 CONNECT request bytes.
pub fn build_request(target_ip: Ipv4Addr, target_port: u16, user_id: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(9 + user_id.len());
    buf.push(0x04); // VER
    buf.push(0x01); // CMD = CONNECT
    buf.extend_from_slice(&target_port.to_be_bytes());
    buf.extend_from_slice(&target_ip.octets());
    buf.extend_from_slice(user_id.as_bytes());
    buf.push(0x00); // NULL terminator
    buf
}

/// Parses the 8-byte SOCKS4 response.
pub fn parse_response(response: &[u8; 8]) -> Result<(), ProxyError> {
    match response[1] {
        0x5A => Ok(()),
        0x5B => Err(ProxyError::RequestRejected(
            "request rejected or failed".into(),
        )),
        0x5C => Err(ProxyError::RequestRejected(
            "request rejected: client identd unreachable".into(),
        )),
        0x5D => Err(ProxyError::RequestRejected(
            "request rejected: identd reports different user-id".into(),
        )),
        code => Err(ProxyError::RequestRejected(format!(
            "SOCKS4 unknown status: {code:#04x}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_bytes() {
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        let request = build_request(ip, 80, "user");

        assert_eq!(request[0], 0x04); // VER
        assert_eq!(request[1], 0x01); // CMD
        assert_eq!(&request[2..4], &80u16.to_be_bytes()); // PORT
        assert_eq!(&request[4..8], &[192, 168, 1, 1]); // IP
        assert_eq!(&request[8..12], b"user"); // USERID
        assert_eq!(request[12], 0x00); // NULL
        assert_eq!(request.len(), 13);
    }

    #[test]
    fn build_request_empty_userid() {
        let ip = Ipv4Addr::new(10, 0, 0, 1);
        let request = build_request(ip, 443, "");

        assert_eq!(request.len(), 9);
        assert_eq!(request[8], 0x00);
    }

    #[test]
    fn parse_response_success() {
        let response = [0x00, 0x5A, 0x00, 0x50, 192, 168, 1, 1];
        assert!(parse_response(&response).is_ok());
    }

    #[test]
    fn parse_response_rejected() {
        let response = [0x00, 0x5B, 0x00, 0x00, 0, 0, 0, 0];
        assert!(parse_response(&response).is_err());
    }

    #[test]
    fn parse_response_identd_unreachable() {
        let response = [0x00, 0x5C, 0x00, 0x00, 0, 0, 0, 0];
        let err = parse_response(&response).unwrap_err();
        assert!(err.to_string().contains("identd unreachable"));
    }

    #[test]
    fn parse_response_identd_mismatch() {
        let response = [0x00, 0x5D, 0x00, 0x00, 0, 0, 0, 0];
        let err = parse_response(&response).unwrap_err();
        assert!(err.to_string().contains("different user-id"));
    }

    #[test]
    fn parse_response_unknown_code() {
        let response = [0x00, 0xFF, 0x00, 0x00, 0, 0, 0, 0];
        let err = parse_response(&response).unwrap_err();
        assert!(err.to_string().contains("unknown status"));
    }
}
