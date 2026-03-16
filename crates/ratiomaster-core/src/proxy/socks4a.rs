/// SOCKS4a proxy client implementation.
///
/// Extension of SOCKS4 that supports hostname-based connections.
/// Instead of a real destination IP, the client sends `0.0.0.x` (where x > 0)
/// and appends the hostname after the USERID+NULL.
///
/// ```text
/// Client -> Proxy:
///   VER(0x04) | CMD(0x01) | DSTPORT(2) | DSTIP(0.0.0.x) | USERID | NULL | HOSTNAME | NULL
///
/// Proxy -> Client:
///   Same 8-byte response as SOCKS4
/// ```
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::ProxyError;

/// Performs SOCKS4a handshake on an already-connected stream.
///
/// Unlike SOCKS4, the hostname is sent to the proxy for resolution.
pub async fn handshake(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
    user_id: &str,
) -> Result<(), ProxyError> {
    let request = build_request(target_host, target_port, user_id);
    stream.write_all(&request).await?;
    stream.flush().await?;

    let mut response = [0u8; 8];
    stream.read_exact(&mut response).await?;

    // SOCKS4a uses the same response format as SOCKS4
    super::socks4::parse_response(&response)
}

/// Builds the SOCKS4a CONNECT request bytes.
pub fn build_request(target_host: &str, target_port: u16, user_id: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(10 + user_id.len() + target_host.len());
    buf.push(0x04); // VER
    buf.push(0x01); // CMD = CONNECT
    buf.extend_from_slice(&target_port.to_be_bytes());
    // DSTIP = 0.0.0.1 — signals hostname mode
    buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    buf.extend_from_slice(user_id.as_bytes());
    buf.push(0x00); // NULL after USERID
    buf.extend_from_slice(target_host.as_bytes());
    buf.push(0x00); // NULL after HOSTNAME
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_bytes() {
        let request = build_request("example.com", 80, "user");

        assert_eq!(request[0], 0x04); // VER
        assert_eq!(request[1], 0x01); // CMD
        assert_eq!(&request[2..4], &80u16.to_be_bytes()); // PORT
        assert_eq!(&request[4..8], &[0, 0, 0, 1]); // Hostname mode marker
        assert_eq!(&request[8..12], b"user"); // USERID
        assert_eq!(request[12], 0x00); // NULL after USERID
        assert_eq!(&request[13..24], b"example.com"); // HOSTNAME
        assert_eq!(request[24], 0x00); // NULL after HOSTNAME
        assert_eq!(request.len(), 25);
    }

    #[test]
    fn build_request_empty_userid() {
        let request = build_request("tracker.example.org", 6969, "");

        assert_eq!(request[0], 0x04);
        assert_eq!(&request[4..8], &[0, 0, 0, 1]);
        assert_eq!(request[8], 0x00); // NULL after empty USERID
        let host_start = 9;
        let host_end = host_start + "tracker.example.org".len();
        assert_eq!(&request[host_start..host_end], b"tracker.example.org");
        assert_eq!(request[host_end], 0x00);
    }

    #[test]
    fn hostname_mode_ip_format() {
        let request = build_request("test.com", 443, "");
        // Bytes 4-7 must be 0.0.0.x where x > 0
        assert_eq!(request[4], 0x00);
        assert_eq!(request[5], 0x00);
        assert_eq!(request[6], 0x00);
        assert!(request[7] > 0, "last byte must be > 0 for hostname mode");
    }
}
