/// TCP BitTorrent handshake listener.
///
/// Listens on a port and responds to incoming BitTorrent protocol handshakes
/// that contain a matching info_hash. This makes the client appear as a real
/// peer to other clients in the swarm.
use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{debug, warn};

/// The BitTorrent protocol handshake header.
const PROTOCOL_STRING: &[u8] = b"BitTorrent protocol";
/// Total handshake length: 1 (pstrlen) + 19 (pstr) + 8 (reserved) + 20 (info_hash) + 20 (peer_id) = 68
const HANDSHAKE_LEN: usize = 68;

/// Starts a TCP listener that responds to BitTorrent handshakes.
///
/// Runs as a background task until the shutdown signal is received.
/// Returns the local address the listener is bound to.
pub async fn start_listener(
    port: u16,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    mut shutdown: watch::Receiver<bool>,
) -> std::io::Result<SocketAddr> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    let local_addr = listener.local_addr()?;

    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((mut stream, addr)) => {
                            let ih = info_hash;
                            let pid = peer_id;
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(&mut stream, addr, &ih, &pid).await {
                                    debug!("handshake error from {addr}: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            warn!("listener accept error: {e}");
                        }
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        debug!("handshake listener shutting down");
                        break;
                    }
                }
            }
        }
    });

    Ok(local_addr)
}

async fn handle_connection(
    stream: &mut tokio::net::TcpStream,
    addr: SocketAddr,
    info_hash: &[u8; 20],
    peer_id: &[u8; 20],
) -> std::io::Result<()> {
    let mut buf = [0u8; HANDSHAKE_LEN];

    let read = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        stream.read_exact(&mut buf).await
    })
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "handshake read timeout"))?;
    read?;

    // Validate handshake
    if !is_valid_handshake(&buf, info_hash) {
        debug!("invalid handshake from {addr}");
        return Ok(());
    }

    debug!("valid handshake from {addr}, sending response");

    // Build and send response handshake
    let response = build_handshake(info_hash, peer_id);
    stream.write_all(&response).await?;
    stream.flush().await?;

    Ok(())
}

/// Validates an incoming BitTorrent handshake.
fn is_valid_handshake(data: &[u8; HANDSHAKE_LEN], expected_info_hash: &[u8; 20]) -> bool {
    // First byte is protocol string length (should be 19)
    if data[0] != 19 {
        return false;
    }

    // Bytes 1..20 should be "BitTorrent protocol"
    if &data[1..20] != PROTOCOL_STRING {
        return false;
    }

    // Bytes 28..48 are the info_hash
    &data[28..48] == expected_info_hash
}

/// Builds a BitTorrent protocol handshake message.
fn build_handshake(info_hash: &[u8; 20], peer_id: &[u8; 20]) -> [u8; HANDSHAKE_LEN] {
    let mut handshake = [0u8; HANDSHAKE_LEN];
    handshake[0] = 19;
    handshake[1..20].copy_from_slice(PROTOCOL_STRING);
    // bytes 20..28 are reserved (all zeros)
    handshake[28..48].copy_from_slice(info_hash);
    handshake[48..68].copy_from_slice(peer_id);
    handshake
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_info_hash() -> [u8; 20] {
        [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14,
        ]
    }

    fn test_peer_id() -> [u8; 20] {
        *b"-UT3320-abcdefghijkl"
    }

    #[test]
    fn build_handshake_format() {
        let ih = test_info_hash();
        let pid = test_peer_id();
        let hs = build_handshake(&ih, &pid);

        assert_eq!(hs[0], 19);
        assert_eq!(&hs[1..20], PROTOCOL_STRING);
        assert_eq!(&hs[20..28], &[0u8; 8]); // reserved bytes
        assert_eq!(&hs[28..48], &ih);
        assert_eq!(&hs[48..68], &pid);
    }

    #[test]
    fn valid_handshake_matching_hash() {
        let ih = test_info_hash();
        let pid = test_peer_id();
        let hs = build_handshake(&ih, &pid);
        assert!(is_valid_handshake(&hs, &ih));
    }

    #[test]
    fn invalid_handshake_wrong_hash() {
        let ih = test_info_hash();
        let pid = test_peer_id();
        let hs = build_handshake(&ih, &pid);
        let wrong_hash = [0xFFu8; 20];
        assert!(!is_valid_handshake(&hs, &wrong_hash));
    }

    #[test]
    fn invalid_handshake_wrong_protocol_length() {
        let ih = test_info_hash();
        let pid = test_peer_id();
        let mut hs = build_handshake(&ih, &pid);
        hs[0] = 18; // wrong length
        assert!(!is_valid_handshake(&hs, &ih));
    }

    #[test]
    fn invalid_handshake_wrong_protocol_string() {
        let ih = test_info_hash();
        let pid = test_peer_id();
        let mut hs = build_handshake(&ih, &pid);
        hs[1] = b'X'; // corrupt protocol string
        assert!(!is_valid_handshake(&hs, &ih));
    }

    async fn start_test_listener(
        info_hash: [u8; 20],
        peer_id: [u8; 20],
        shutdown: watch::Receiver<bool>,
    ) -> std::io::Result<SocketAddr> {
        let listener = TcpListener::bind(("127.0.0.1", 0u16)).await?;
        let local_addr = listener.local_addr()?;
        tokio::spawn(async move {
            let mut shutdown = shutdown;
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((mut stream, addr)) => {
                                let ih = info_hash;
                                let pid = peer_id;
                                tokio::spawn(async move {
                                    let _ = handle_connection(&mut stream, addr, &ih, &pid).await;
                                });
                            }
                            Err(_) => break,
                        }
                    }
                    _ = shutdown.changed() => break,
                }
            }
        });
        Ok(local_addr)
    }

    #[tokio::test]
    async fn listener_accepts_handshake() {
        let ih = test_info_hash();
        let pid = test_peer_id();
        let (tx, rx) = watch::channel(false);

        let addr = start_test_listener(ih, pid, rx).await.unwrap();

        // Connect and send handshake
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let outgoing = build_handshake(&ih, b"-XX0000-123456789012");
        client.write_all(&outgoing).await.unwrap();

        // Read response handshake
        let mut buf = [0u8; HANDSHAKE_LEN];
        client.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf[0], 19);
        assert_eq!(&buf[1..20], PROTOCOL_STRING);
        assert_eq!(&buf[28..48], &ih);
        assert_eq!(&buf[48..68], &pid);

        // Shutdown
        let _ = tx.send(true);
    }

    #[tokio::test]
    async fn listener_rejects_wrong_hash() {
        let ih = test_info_hash();
        let pid = test_peer_id();
        let (tx, rx) = watch::channel(false);

        let addr = start_test_listener(ih, pid, rx).await.unwrap();

        // Connect with wrong info_hash
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let wrong_hash = [0xFFu8; 20];
        let outgoing = build_handshake(&wrong_hash, b"-XX0000-123456789012");
        client.write_all(&outgoing).await.unwrap();

        // The listener should not send a response — connection should close
        let mut buf = [0u8; HANDSHAKE_LEN];
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            client.read_exact(&mut buf),
        )
        .await;

        // Either timeout or connection closed without full handshake
        assert!(result.is_err() || result.unwrap().is_err());

        let _ = tx.send(true);
    }

    #[tokio::test]
    async fn listener_shutdown() {
        let ih = test_info_hash();
        let pid = test_peer_id();
        let (tx, rx) = watch::channel(false);

        let _addr = start_test_listener(ih, pid, rx).await.unwrap();

        // Shutdown should complete without hanging
        let _ = tx.send(true);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
