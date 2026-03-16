use super::value::BValue;

/// Encodes a `BValue` into its BEncoded byte representation.
///
/// # Examples
///
/// ```
/// use ratiomaster_core::bencode::{BValue, encode};
///
/// let encoded = encode(&BValue::Integer(42));
/// assert_eq!(encoded, b"i42e");
/// ```
pub fn encode(value: &BValue) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_into(value, &mut buf);
    buf
}

/// Encodes a `BValue` into an existing byte buffer.
pub fn encode_into(value: &BValue, buf: &mut Vec<u8>) {
    match value {
        BValue::String(bytes) => {
            buf.extend_from_slice(bytes.len().to_string().as_bytes());
            buf.push(b':');
            buf.extend_from_slice(bytes);
        }
        BValue::Integer(n) => {
            buf.push(b'i');
            buf.extend_from_slice(n.to_string().as_bytes());
            buf.push(b'e');
        }
        BValue::List(items) => {
            buf.push(b'l');
            for item in items {
                encode_into(item, buf);
            }
            buf.push(b'e');
        }
        BValue::Dict(map) => {
            buf.push(b'd');
            // BTreeMap iterates in sorted order, which is required by the BEncode spec
            for (key, val) in map {
                buf.extend_from_slice(key.len().to_string().as_bytes());
                buf.push(b':');
                buf.extend_from_slice(key);
                encode_into(val, buf);
            }
            buf.push(b'e');
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn encode_integer() {
        assert_eq!(encode(&BValue::Integer(42)), b"i42e");
        assert_eq!(encode(&BValue::Integer(0)), b"i0e");
        assert_eq!(encode(&BValue::Integer(-1)), b"i-1e");
    }

    #[test]
    fn encode_string() {
        assert_eq!(encode(&BValue::String(b"spam".to_vec())), b"4:spam");
        assert_eq!(encode(&BValue::String(b"".to_vec())), b"0:");
    }

    #[test]
    fn encode_list() {
        let list = BValue::List(vec![BValue::String(b"spam".to_vec()), BValue::Integer(42)]);
        assert_eq!(encode(&list), b"l4:spami42ee");
    }

    #[test]
    fn encode_empty_list() {
        assert_eq!(encode(&BValue::List(vec![])), b"le");
    }

    #[test]
    fn encode_dict() {
        let mut map = BTreeMap::new();
        map.insert(b"cow".to_vec(), BValue::String(b"moo".to_vec()));
        map.insert(b"spam".to_vec(), BValue::String(b"eggs".to_vec()));
        let dict = BValue::Dict(map);
        assert_eq!(encode(&dict), b"d3:cow3:moo4:spam4:eggse");
    }

    #[test]
    fn encode_empty_dict() {
        assert_eq!(encode(&BValue::Dict(BTreeMap::new())), b"de");
    }

    #[test]
    fn encode_binary_string() {
        let val = BValue::String(vec![0x00, 0xff, 0x80]);
        let encoded = encode(&val);
        assert_eq!(&encoded[..2], b"3:");
        assert_eq!(&encoded[2..], &[0x00, 0xff, 0x80]);
    }

    #[test]
    fn roundtrip_complex() {
        use crate::bencode::decode;

        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(12345));
        info.insert(b"name".to_vec(), BValue::String(b"test.txt".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(262144));
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xab; 20]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let original = BValue::Dict(root);
        let encoded = encode(&original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);

        // Double-check: re-encoding produces identical bytes
        let re_encoded = encode(&decoded);
        assert_eq!(re_encoded, encoded);
    }

    #[test]
    fn encode_deeply_nested() {
        let inner = BValue::Dict(BTreeMap::new());
        let mut level1 = BTreeMap::new();
        level1.insert(b"a".to_vec(), inner);
        let list = BValue::List(vec![BValue::Dict(level1)]);
        let mut level2 = BTreeMap::new();
        level2.insert(b"b".to_vec(), list);
        let val = BValue::Dict(level2);

        let encoded = encode(&val);
        let decoded = crate::bencode::decode(&encoded).unwrap();
        assert_eq!(decoded, val);
    }

    #[test]
    fn encode_large_integer_values() {
        assert_eq!(
            encode(&BValue::Integer(i64::MAX)),
            format!("i{}e", i64::MAX).into_bytes()
        );
        assert_eq!(
            encode(&BValue::Integer(i64::MIN)),
            format!("i{}e", i64::MIN).into_bytes()
        );
    }

    #[test]
    fn encode_zero_length_string() {
        assert_eq!(encode(&BValue::String(vec![])), b"0:");
    }

    #[test]
    fn encode_all_256_byte_values() {
        let all_bytes: Vec<u8> = (0u16..256).map(|b| b as u8).collect();
        let val = BValue::String(all_bytes.clone());
        let encoded = encode(&val);
        let decoded = crate::bencode::decode(&encoded).unwrap();
        assert_eq!(decoded.as_bytes().unwrap(), &all_bytes[..]);
    }

    #[test]
    fn encode_dict_many_keys() {
        let mut map = BTreeMap::new();
        for i in 0..55u32 {
            map.insert(format!("k{:03}", i).into_bytes(), BValue::Integer(i as i64));
        }
        let val = BValue::Dict(map);
        let encoded = encode(&val);
        let decoded = crate::bencode::decode(&encoded).unwrap();
        assert_eq!(decoded.as_dict().unwrap().len(), 55);
    }

    #[test]
    fn encode_list_mixed_types() {
        let mut dict = BTreeMap::new();
        dict.insert(b"x".to_vec(), BValue::Integer(1));
        let list = BValue::List(vec![
            BValue::String(b"hello".to_vec()),
            BValue::Integer(-42),
            BValue::List(vec![]),
            BValue::Dict(dict),
        ]);
        let encoded = encode(&list);
        let decoded = crate::bencode::decode(&encoded).unwrap();
        assert_eq!(decoded, list);
    }
}
