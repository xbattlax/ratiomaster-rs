const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";
const HEX_LOWER: &[u8; 16] = b"0123456789abcdef";

/// URL-encodes a byte slice using percent-encoding.
///
/// Bytes that are unreserved (alphanumeric plus `-`, `_`, `.`, `~`) pass through
/// unencoded. All other bytes are encoded as `%XX` where `XX` is the uppercase
/// hex representation by default.
///
/// # Arguments
///
/// * `bytes` - The binary data to encode (e.g., a 20-byte info_hash).
/// * `uppercase` - If `true`, hex digits are uppercase (`%2F`). If `false`, lowercase (`%2f`).
///
/// # Examples
///
/// ```
/// use ratiomaster_core::encoding::url_encode;
///
/// let hash = [0x12, 0xab, 0x34];
/// assert_eq!(url_encode(&hash, true), "%12%AB4");
/// assert_eq!(url_encode(&hash, false), "%12%ab4");
/// ```
pub fn url_encode(bytes: &[u8], uppercase: bool) -> String {
    let hex = if uppercase { HEX_UPPER } else { HEX_LOWER };
    let mut result = String::with_capacity(bytes.len() * 3);

    for &byte in bytes {
        if is_unreserved(byte) {
            result.push(byte as char);
        } else {
            result.push('%');
            result.push(hex[(byte >> 4) as usize] as char);
            result.push(hex[(byte & 0x0F) as usize] as char);
        }
    }

    result
}

/// Returns `true` if the byte is an unreserved character per RFC 3986.
///
/// Unreserved characters are: `A-Z`, `a-z`, `0-9`, `-`, `_`, `.`, `~`
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_all_zeros() {
        let input = [0u8; 3];
        assert_eq!(url_encode(&input, true), "%00%00%00");
        assert_eq!(url_encode(&input, false), "%00%00%00");
    }

    #[test]
    fn encode_all_unreserved() {
        let input = b"abc123-_.~XYZ";
        assert_eq!(url_encode(input, true), "abc123-_.~XYZ");
        assert_eq!(url_encode(input, false), "abc123-_.~XYZ");
    }

    #[test]
    fn encode_mixed() {
        // 0x41 = 'A' (unreserved), 0xFF (reserved), 0x30 = '0' (unreserved)
        let input = [0x41, 0xFF, 0x30];
        assert_eq!(url_encode(&input, true), "A%FF0");
        assert_eq!(url_encode(&input, false), "A%ff0");
    }

    #[test]
    fn encode_info_hash_known_vector() {
        // A realistic 20-byte info_hash
        let hash: [u8; 20] = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0, 0x12, 0x34, 0x56, 0x78,
        ];
        let encoded_upper = url_encode(&hash, true);
        let encoded_lower = url_encode(&hash, false);

        assert_eq!(encoded_upper, "%124Vx%9A%BC%DE%F0%124Vx%9A%BC%DE%F0%124Vx");
        assert_eq!(encoded_lower, "%124Vx%9a%bc%de%f0%124Vx%9a%bc%de%f0%124Vx");
    }

    #[test]
    fn encode_empty() {
        assert_eq!(url_encode(&[], true), "");
    }

    #[test]
    fn encode_space_and_special() {
        let input = b" /&=";
        assert_eq!(url_encode(input, true), "%20%2F%26%3D");
        assert_eq!(url_encode(input, false), "%20%2f%26%3d");
    }

    #[test]
    fn uppercase_vs_lowercase() {
        let input = [0xAB];
        assert_eq!(url_encode(&input, true), "%AB");
        assert_eq!(url_encode(&input, false), "%ab");
    }

    #[test]
    fn encode_all_byte_values_unreserved_passthrough() {
        for byte in 0u8..=255 {
            let encoded = url_encode(&[byte], true);
            if is_unreserved(byte) {
                assert_eq!(encoded.len(), 1, "byte {byte:#04x} should pass through");
                assert_eq!(encoded.as_bytes()[0], byte);
            } else {
                assert_eq!(
                    encoded.len(),
                    3,
                    "byte {byte:#04x} should be percent-encoded"
                );
                assert!(encoded.starts_with('%'));
            }
        }
    }
}
