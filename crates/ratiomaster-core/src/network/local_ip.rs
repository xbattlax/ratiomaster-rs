/// Detects the primary local IP address of the machine.
///
/// Uses a UDP socket trick: connects to a public address (without sending data)
/// to determine which local interface would be used. Falls back to `0.0.0.0`
/// if detection fails.
use std::net::{IpAddr, Ipv4Addr, UdpSocket};

/// Returns the primary non-loopback local IPv4 address.
///
/// This works by creating a UDP socket and "connecting" it to a public IP.
/// The OS routing table determines which local address to bind, revealing
/// the primary outbound interface. No data is actually sent.
///
/// Returns `0.0.0.0` if detection fails.
pub fn detect_local_ip() -> IpAddr {
    detect_local_ip_inner().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

fn detect_local_ip_inner() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    // Connect to a public DNS server — no data is sent over UDP
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    let ip = addr.ip();

    if ip.is_loopback() || ip.is_unspecified() {
        None
    } else {
        Some(ip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_non_loopback() {
        let ip = detect_local_ip();
        // Should not be loopback (unless running in a very unusual environment)
        assert!(!ip.is_loopback(), "expected non-loopback IP, got {ip}");
    }

    #[test]
    fn detect_returns_valid_ip() {
        let ip = detect_local_ip();
        // Should be either a real IP or the fallback 0.0.0.0
        match ip {
            IpAddr::V4(v4) => {
                // Either a real private/public IP or the fallback
                assert!(!v4.is_loopback(), "should not return loopback, got {v4}");
            }
            IpAddr::V6(v6) => {
                assert!(!v6.is_loopback(), "should not return loopback, got {v6}");
            }
        }
    }
}
