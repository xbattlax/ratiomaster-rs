/// SOCKS5 proxy client implementation (RFC 1928).
///
/// Three-phase protocol:
/// 1. Greeting: client sends supported auth methods, server picks one
/// 2. Authentication: if required (RFC 1929 username/password)
/// 3. Connect: client sends target address, server establishes tunnel
///
/// Supported address types: IPv4 (0x01), Domain (0x03), IPv6 (0x04).
use std::net::{Ipv4Addr, Ipv6Addr};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::ProxyError;

/// SOCKS5 authentication credentials.
///
/// Password is zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"***")
            .finish()
    }
}

/// Target address for SOCKS5 CONNECT.
#[derive(Debug, Clone)]
pub enum Address {
    Ipv4(Ipv4Addr),
    Domain(String),
    Ipv6(Ipv6Addr),
}

/// Performs full SOCKS5 handshake: greeting, optional auth, and connect.
pub async fn handshake(
    stream: &mut TcpStream,
    target: &Address,
    target_port: u16,
    credentials: Option<&Credentials>,
) -> Result<(), ProxyError> {
    // Phase 1: Greeting
    let greeting = build_greeting(credentials.is_some());
    stream.write_all(&greeting).await?;
    stream.flush().await?;

    let mut greeting_resp = [0u8; 2];
    stream.read_exact(&mut greeting_resp).await?;
    parse_greeting_response(&greeting_resp, credentials.is_some())?;

    // Phase 2: Authentication (if server selected 0x02)
    if greeting_resp[1] == 0x02 {
        let creds = credentials.ok_or_else(|| {
            ProxyError::AuthenticationFailed(
                "server requires auth but no credentials provided".into(),
            )
        })?;
        let auth_req = build_auth_request(creds)?;
        stream.write_all(&auth_req).await?;
        stream.flush().await?;

        let mut auth_resp = [0u8; 2];
        stream.read_exact(&mut auth_resp).await?;
        parse_auth_response(&auth_resp)?;
    }

    // Phase 3: Connect
    let connect_req = build_connect_request(target, target_port)?;
    stream.write_all(&connect_req).await?;
    stream.flush().await?;

    read_connect_response(stream).await
}

/// Builds the SOCKS5 greeting message.
///
/// ```text
/// VER(0x05) | NMETHODS(1-2) | METHODS...
/// ```
pub fn build_greeting(with_auth: bool) -> Vec<u8> {
    if with_auth {
        vec![0x05, 0x02, 0x00, 0x02] // No auth + username/password
    } else {
        vec![0x05, 0x01, 0x00] // No auth only
    }
}

/// Parses the server's greeting response.
pub fn parse_greeting_response(response: &[u8; 2], auth_offered: bool) -> Result<(), ProxyError> {
    if response[0] != 0x05 {
        return Err(ProxyError::ProtocolError(format!(
            "expected SOCKS5 version 0x05, got {:#04x}",
            response[0]
        )));
    }

    match response[1] {
        0x00 => Ok(()),                 // No authentication
        0x02 if auth_offered => Ok(()), // Username/password accepted
        0x02 => Err(ProxyError::AuthenticationFailed(
            "server requires authentication but none provided".into(),
        )),
        0xFF => Err(ProxyError::AuthenticationFailed(
            "no acceptable authentication method".into(),
        )),
        method => Err(ProxyError::ProtocolError(format!(
            "unsupported auth method: {method:#04x}"
        ))),
    }
}

/// Builds RFC 1929 username/password authentication request.
///
/// ```text
/// VER(0x01) | ULEN(1) | UNAME(1-255) | PLEN(1) | PASSWD(1-255)
/// ```
pub fn build_auth_request(credentials: &Credentials) -> Result<Vec<u8>, ProxyError> {
    let ulen = credentials.username.len();
    let plen = credentials.password.len();
    if ulen > 255 {
        return Err(ProxyError::ProtocolError(format!(
            "SOCKS5 username exceeds 255 bytes ({ulen} bytes)"
        )));
    }
    if plen > 255 {
        return Err(ProxyError::ProtocolError(format!(
            "SOCKS5 password exceeds 255 bytes ({plen} bytes)"
        )));
    }
    let mut buf = Vec::with_capacity(3 + ulen + plen);
    buf.push(0x01); // Auth sub-negotiation version
    buf.push(ulen as u8);
    buf.extend_from_slice(credentials.username.as_bytes());
    buf.push(plen as u8);
    buf.extend_from_slice(credentials.password.as_bytes());
    Ok(buf)
}

/// Parses the authentication response.
pub fn parse_auth_response(response: &[u8; 2]) -> Result<(), ProxyError> {
    if response[1] != 0x00 {
        return Err(ProxyError::AuthenticationFailed(
            "username/password authentication failed".into(),
        ));
    }
    Ok(())
}

/// Builds SOCKS5 CONNECT request.
///
/// ```text
/// VER(0x05) | CMD(0x01) | RSV(0x00) | ATYP | DST.ADDR | DST.PORT
/// ```
pub fn build_connect_request(target: &Address, port: u16) -> Result<Vec<u8>, ProxyError> {
    let mut buf = Vec::new();
    buf.push(0x05); // VER
    buf.push(0x01); // CMD = CONNECT
    buf.push(0x00); // RSV

    match target {
        Address::Ipv4(ip) => {
            buf.push(0x01); // ATYP = IPv4
            buf.extend_from_slice(&ip.octets());
        }
        Address::Domain(host) => {
            let len = host.len();
            if len > 255 {
                return Err(ProxyError::ProtocolError(format!(
                    "SOCKS5 domain exceeds 255 bytes ({len} bytes)"
                )));
            }
            buf.push(0x03); // ATYP = Domain
            buf.push(len as u8);
            buf.extend_from_slice(host.as_bytes());
        }
        Address::Ipv6(ip) => {
            buf.push(0x04); // ATYP = IPv6
            buf.extend_from_slice(&ip.octets());
        }
    }

    buf.extend_from_slice(&port.to_be_bytes());
    Ok(buf)
}

/// Reads and parses the SOCKS5 connect response.
///
/// Response has variable length depending on address type:
/// ```text
/// VER | REP | RSV | ATYP | BND.ADDR | BND.PORT
/// ```
async fn read_connect_response(stream: &mut TcpStream) -> Result<(), ProxyError> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;

    if header[0] != 0x05 {
        return Err(ProxyError::ProtocolError(format!(
            "expected SOCKS5 version, got {:#04x}",
            header[0]
        )));
    }

    if header[1] != 0x00 {
        return Err(ProxyError::RequestRejected(socks5_reply_message(header[1])));
    }

    // Skip the bound address based on ATYP
    match header[3] {
        0x01 => {
            // IPv4: 4 bytes + 2 port
            let mut skip = [0u8; 6];
            stream.read_exact(&mut skip).await?;
        }
        0x03 => {
            // Domain: 1 byte length + domain + 2 port
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await?;
            let mut skip = vec![0u8; len_buf[0] as usize + 2];
            stream.read_exact(&mut skip).await?;
        }
        0x04 => {
            // IPv6: 16 bytes + 2 port
            let mut skip = [0u8; 18];
            stream.read_exact(&mut skip).await?;
        }
        atyp => {
            return Err(ProxyError::ProtocolError(format!(
                "unknown ATYP: {atyp:#04x}"
            )));
        }
    }

    Ok(())
}

fn socks5_reply_message(code: u8) -> String {
    match code {
        0x01 => "general SOCKS server failure".into(),
        0x02 => "connection not allowed by ruleset".into(),
        0x03 => "network unreachable".into(),
        0x04 => "host unreachable".into(),
        0x05 => "connection refused".into(),
        0x06 => "TTL expired".into(),
        0x07 => "command not supported".into(),
        0x08 => "address type not supported".into(),
        _ => format!("unknown reply code: {code:#04x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_no_auth() {
        let msg = build_greeting(false);
        assert_eq!(msg, vec![0x05, 0x01, 0x00]);
    }

    #[test]
    fn greeting_with_auth() {
        let msg = build_greeting(true);
        assert_eq!(msg, vec![0x05, 0x02, 0x00, 0x02]);
    }

    #[test]
    fn parse_greeting_no_auth_ok() {
        assert!(parse_greeting_response(&[0x05, 0x00], false).is_ok());
    }

    #[test]
    fn parse_greeting_auth_selected() {
        assert!(parse_greeting_response(&[0x05, 0x02], true).is_ok());
    }

    #[test]
    fn parse_greeting_auth_required_but_not_offered() {
        assert!(parse_greeting_response(&[0x05, 0x02], false).is_err());
    }

    #[test]
    fn parse_greeting_no_acceptable_method() {
        let result = parse_greeting_response(&[0x05, 0xFF], true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no acceptable"));
    }

    #[test]
    fn parse_greeting_wrong_version() {
        let result = parse_greeting_response(&[0x04, 0x00], false);
        assert!(result.is_err());
    }

    #[test]
    fn auth_request_bytes() {
        let creds = Credentials {
            username: "user".into(),
            password: "pass".into(),
        };
        let req = build_auth_request(&creds).unwrap();
        assert_eq!(req[0], 0x01); // VER
        assert_eq!(req[1], 4); // ULEN
        assert_eq!(&req[2..6], b"user");
        assert_eq!(req[6], 4); // PLEN
        assert_eq!(&req[7..11], b"pass");
    }

    #[test]
    fn auth_request_rejects_long_username() {
        let creds = Credentials {
            username: "x".repeat(256),
            password: "pass".into(),
        };
        assert!(build_auth_request(&creds).is_err());
    }

    #[test]
    fn auth_request_rejects_long_password() {
        let creds = Credentials {
            username: "user".into(),
            password: "x".repeat(256),
        };
        assert!(build_auth_request(&creds).is_err());
    }

    #[test]
    fn auth_response_success() {
        assert!(parse_auth_response(&[0x01, 0x00]).is_ok());
    }

    #[test]
    fn auth_response_failure() {
        assert!(parse_auth_response(&[0x01, 0x01]).is_err());
    }

    #[test]
    fn connect_request_ipv4() {
        let req = build_connect_request(&Address::Ipv4(Ipv4Addr::new(192, 168, 1, 1)), 80).unwrap();
        assert_eq!(req[0], 0x05); // VER
        assert_eq!(req[1], 0x01); // CMD
        assert_eq!(req[2], 0x00); // RSV
        assert_eq!(req[3], 0x01); // ATYP = IPv4
        assert_eq!(&req[4..8], &[192, 168, 1, 1]);
        assert_eq!(&req[8..10], &80u16.to_be_bytes());
    }

    #[test]
    fn connect_request_domain() {
        let req = build_connect_request(&Address::Domain("example.com".into()), 443).unwrap();
        assert_eq!(req[0], 0x05);
        assert_eq!(req[3], 0x03); // ATYP = Domain
        assert_eq!(req[4], 11); // Domain length
        assert_eq!(&req[5..16], b"example.com");
        assert_eq!(&req[16..18], &443u16.to_be_bytes());
    }

    #[test]
    fn connect_request_rejects_long_domain() {
        let long_domain = "x".repeat(256);
        assert!(build_connect_request(&Address::Domain(long_domain), 443).is_err());
    }

    #[test]
    fn connect_request_ipv6() {
        let ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let req = build_connect_request(&Address::Ipv6(ip), 8080).unwrap();
        assert_eq!(req[3], 0x04); // ATYP = IPv6
        assert_eq!(&req[4..20], &ip.octets());
        assert_eq!(&req[20..22], &8080u16.to_be_bytes());
    }

    #[test]
    fn reply_messages() {
        assert_eq!(socks5_reply_message(0x01), "general SOCKS server failure");
        assert_eq!(socks5_reply_message(0x05), "connection refused");
        assert!(socks5_reply_message(0xFE).contains("unknown"));
    }
}
