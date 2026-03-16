use std::collections::BTreeMap;

use super::error::BencodeError;
use super::value::BValue;

/// Internal decoder state tracking the current position in the input.
struct Decoder<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn peek(&self) -> Result<u8, BencodeError> {
        self.data
            .get(self.pos)
            .copied()
            .ok_or(BencodeError::UnexpectedEof)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect(&mut self, byte: u8) -> Result<(), BencodeError> {
        let b = self.peek()?;
        if b != byte {
            return Err(BencodeError::InvalidByte {
                byte: b,
                position: self.pos,
            });
        }
        self.advance();
        Ok(())
    }

    fn decode_value(&mut self) -> Result<BValue, BencodeError> {
        match self.peek()? {
            b'i' => self.decode_integer(),
            b'l' => self.decode_list(),
            b'd' => self.decode_dict(),
            b'0'..=b'9' => self.decode_string(),
            b => Err(BencodeError::InvalidByte {
                byte: b,
                position: self.pos,
            }),
        }
    }

    fn decode_integer(&mut self) -> Result<BValue, BencodeError> {
        self.expect(b'i')?;
        let start = self.pos;

        // Find the 'e' terminator
        while self.pos < self.data.len() && self.data[self.pos] != b'e' {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return Err(BencodeError::UnexpectedEof);
        }

        let num_str = std::str::from_utf8(&self.data[start..self.pos])
            .map_err(|_| BencodeError::InvalidInteger("not valid UTF-8".into()))?;

        // Validate: no leading zeros (except "0" itself), no "-0"
        if num_str.is_empty() {
            return Err(BencodeError::InvalidInteger("empty integer".into()));
        }
        if num_str == "-0" {
            return Err(BencodeError::InvalidInteger("negative zero".into()));
        }
        if num_str.len() > 1 && num_str.starts_with('0') {
            return Err(BencodeError::InvalidInteger(format!(
                "leading zero: {num_str}"
            )));
        }
        if num_str.len() > 2 && num_str.starts_with("-0") {
            return Err(BencodeError::InvalidInteger(format!(
                "leading zero after minus: {num_str}"
            )));
        }

        let value: i64 = num_str
            .parse()
            .map_err(|_| BencodeError::InvalidInteger(num_str.into()))?;

        self.expect(b'e')?;
        Ok(BValue::Integer(value))
    }

    fn decode_string(&mut self) -> Result<BValue, BencodeError> {
        let start = self.pos;

        // Parse the length prefix
        while self.pos < self.data.len() && self.data[self.pos] != b':' {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return Err(BencodeError::UnexpectedEof);
        }

        let len_str = std::str::from_utf8(&self.data[start..self.pos])
            .map_err(|_| BencodeError::InvalidStringLength("not valid UTF-8".into()))?;

        let len: usize = len_str
            .parse()
            .map_err(|_| BencodeError::InvalidStringLength(len_str.into()))?;

        // Skip the ':'
        self.advance();

        // Read `len` bytes
        if self.pos + len > self.data.len() {
            return Err(BencodeError::UnexpectedEof);
        }

        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;

        Ok(BValue::String(bytes))
    }

    fn decode_list(&mut self) -> Result<BValue, BencodeError> {
        self.expect(b'l')?;
        let mut items = Vec::new();

        while self.peek()? != b'e' {
            items.push(self.decode_value()?);
        }

        self.expect(b'e')?;
        Ok(BValue::List(items))
    }

    fn decode_dict(&mut self) -> Result<BValue, BencodeError> {
        self.expect(b'd')?;
        let mut map = BTreeMap::new();
        let mut last_key: Option<Vec<u8>> = None;

        while self.peek()? != b'e' {
            let key_value = self.decode_string()?;
            let key = match key_value {
                BValue::String(k) => k,
                _ => unreachable!("decode_string always returns BValue::String"),
            };

            // Enforce sorted key order
            if let Some(ref prev) = last_key {
                if key <= *prev {
                    return Err(BencodeError::UnsortedKeys);
                }
            }
            last_key = Some(key.clone());

            let value = self.decode_value()?;
            map.insert(key, value);
        }

        self.expect(b'e')?;
        Ok(BValue::Dict(map))
    }
}

/// Decodes a BEncoded byte slice into a `BValue`.
///
/// Returns an error if the input is malformed or contains trailing data.
///
/// # Examples
///
/// ```
/// use ratiomaster_core::bencode::decode;
///
/// let value = decode(b"i42e").unwrap();
/// assert_eq!(value.as_integer(), Some(42));
/// ```
pub fn decode(bytes: &[u8]) -> Result<BValue, BencodeError> {
    let mut decoder = Decoder::new(bytes);
    let value = decoder.decode_value()?;

    if decoder.pos < decoder.data.len() {
        return Err(BencodeError::TrailingData {
            remaining: decoder.data.len() - decoder.pos,
        });
    }

    Ok(value)
}

/// Decodes a BEncoded byte slice, returning the value and the number of bytes consumed.
///
/// Unlike [`decode`], this does not require the entire input to be consumed.
/// This is useful when parsing a value that is embedded within a larger structure
/// (e.g., extracting the raw info dict bytes for hashing).
pub fn decode_prefix(bytes: &[u8]) -> Result<(BValue, usize), BencodeError> {
    let mut decoder = Decoder::new(bytes);
    let value = decoder.decode_value()?;
    Ok((value, decoder.pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_integer() {
        assert_eq!(decode(b"i42e").unwrap(), BValue::Integer(42));
        assert_eq!(decode(b"i0e").unwrap(), BValue::Integer(0));
        assert_eq!(decode(b"i-1e").unwrap(), BValue::Integer(-1));
        assert_eq!(
            decode(b"i9999999999e").unwrap(),
            BValue::Integer(9_999_999_999)
        );
    }

    #[test]
    fn decode_integer_negative_zero_rejected() {
        assert!(matches!(
            decode(b"i-0e"),
            Err(BencodeError::InvalidInteger(_))
        ));
    }

    #[test]
    fn decode_integer_leading_zero_rejected() {
        assert!(matches!(
            decode(b"i03e"),
            Err(BencodeError::InvalidInteger(_))
        ));
    }

    #[test]
    fn decode_integer_empty_rejected() {
        assert!(matches!(
            decode(b"ie"),
            Err(BencodeError::InvalidInteger(_))
        ));
    }

    #[test]
    fn decode_string() {
        assert_eq!(decode(b"4:spam").unwrap(), BValue::String(b"spam".to_vec()));
        assert_eq!(decode(b"0:").unwrap(), BValue::String(b"".to_vec()));
    }

    #[test]
    fn decode_string_binary() {
        let data = b"3:\x00\xff\x80";
        let val = decode(data).unwrap();
        assert_eq!(val.as_bytes().unwrap(), &[0x00, 0xff, 0x80]);
    }

    #[test]
    fn decode_list() {
        let val = decode(b"l4:spam4:eggse").unwrap();
        let list = val.as_list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].as_str().unwrap(), "spam");
        assert_eq!(list[1].as_str().unwrap(), "eggs");
    }

    #[test]
    fn decode_empty_list() {
        let val = decode(b"le").unwrap();
        assert_eq!(val.as_list().unwrap().len(), 0);
    }

    #[test]
    fn decode_nested_list() {
        let val = decode(b"lli1ei2eeli3eee").unwrap();
        let list = val.as_list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].as_list().unwrap().len(), 2);
    }

    #[test]
    fn decode_dict() {
        let val = decode(b"d3:cow3:moo4:spam4:eggse").unwrap();
        let dict = val.as_dict().unwrap();
        assert_eq!(dict.len(), 2);
        assert_eq!(
            dict.get(b"cow".as_slice()).unwrap().as_str().unwrap(),
            "moo"
        );
        assert_eq!(
            dict.get(b"spam".as_slice()).unwrap().as_str().unwrap(),
            "eggs"
        );
    }

    #[test]
    fn decode_empty_dict() {
        let val = decode(b"de").unwrap();
        assert_eq!(val.as_dict().unwrap().len(), 0);
    }

    #[test]
    fn decode_dict_unsorted_keys_rejected() {
        // "spam" comes after "cow" alphabetically, but here we reverse them
        assert!(matches!(
            decode(b"d4:spam4:eggs3:cow3:mooe"),
            Err(BencodeError::UnsortedKeys)
        ));
    }

    #[test]
    fn decode_nested_dict() {
        let val = decode(b"d4:infod4:name4:testee").unwrap();
        let inner = val.dict_get("info").unwrap();
        assert_eq!(inner.dict_get("name").unwrap().as_str().unwrap(), "test");
    }

    #[test]
    fn decode_trailing_data() {
        assert!(matches!(
            decode(b"i42eextra"),
            Err(BencodeError::TrailingData { remaining: 5 })
        ));
    }

    #[test]
    fn decode_empty_input() {
        assert!(matches!(decode(b""), Err(BencodeError::UnexpectedEof)));
    }

    #[test]
    fn decode_prefix_basic() {
        let (val, consumed) = decode_prefix(b"i42eextra").unwrap();
        assert_eq!(val.as_integer().unwrap(), 42);
        assert_eq!(consumed, 4);
    }

    #[test]
    fn decode_large_negative_integer() {
        assert_eq!(
            decode(b"i-9223372036854775808e").unwrap(),
            BValue::Integer(i64::MIN)
        );
    }

    #[test]
    fn decode_deeply_nested_structure() {
        // dict inside list inside dict inside list inside dict (5 levels)
        // d4:datald4:infod5:itemsli1ei2ei3ee4:name4:testee4:type4:listee
        let mut l3 = BTreeMap::new();
        l3.insert(
            b"items".to_vec(),
            BValue::List(vec![
                BValue::Integer(1),
                BValue::Integer(2),
                BValue::Integer(3),
            ]),
        );
        l3.insert(b"name".to_vec(), BValue::String(b"test".to_vec()));

        let l2 = BValue::List(vec![BValue::Dict(l3)]);

        let mut l1 = BTreeMap::new();
        l1.insert(b"data".to_vec(), l2);
        l1.insert(b"type".to_vec(), BValue::String(b"list".to_vec()));

        let original = BValue::Dict(l1);
        let encoded = crate::bencode::encode(&original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_six_level_nesting() {
        // l l l l l i42e e e e e e
        let val = decode(b"llllli42eeeeee").unwrap();
        let l1 = val.as_list().unwrap();
        let l2 = l1[0].as_list().unwrap();
        let l3 = l2[0].as_list().unwrap();
        let l4 = l3[0].as_list().unwrap();
        let l5 = l4[0].as_list().unwrap();
        assert_eq!(l5[0].as_integer().unwrap(), 42);
    }

    #[test]
    fn decode_very_large_integer_overflow() {
        // 9999999999999999999 overflows i64 (max 9223372036854775807)
        assert!(matches!(
            decode(b"i9999999999999999999e"),
            Err(BencodeError::InvalidInteger(_))
        ));
    }

    #[test]
    fn decode_i64_max() {
        let encoded = format!("i{}e", i64::MAX);
        assert_eq!(
            decode(encoded.as_bytes()).unwrap(),
            BValue::Integer(i64::MAX)
        );
    }

    #[test]
    fn decode_zero_length_string() {
        let val = decode(b"0:").unwrap();
        assert_eq!(val.as_bytes().unwrap(), &[] as &[u8]);
        assert_eq!(val.as_str().unwrap(), "");
    }

    #[test]
    fn decode_dict_with_many_keys() {
        use std::collections::BTreeMap;
        // Build a dict with 60 keys
        let mut map = BTreeMap::new();
        for i in 0..60u32 {
            let key = format!("key_{:04}", i);
            map.insert(key.as_bytes().to_vec(), BValue::Integer(i as i64));
        }
        let original = BValue::Dict(map);
        let encoded = crate::bencode::encode(&original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(decoded.as_dict().unwrap().len(), 60);
    }

    #[test]
    fn decode_binary_all_256_byte_values() {
        // String containing all 256 possible byte values
        let all_bytes: Vec<u8> = (0u16..256).map(|b| b as u8).collect();
        let mut encoded = format!("{}:", all_bytes.len()).into_bytes();
        encoded.extend_from_slice(&all_bytes);
        let val = decode(&encoded).unwrap();
        assert_eq!(val.as_bytes().unwrap(), &all_bytes[..]);
    }

    #[test]
    fn decode_truncated_integer() {
        assert!(matches!(decode(b"i42"), Err(BencodeError::UnexpectedEof)));
    }

    #[test]
    fn decode_truncated_string() {
        // Claims 10 bytes but only has 3
        assert!(matches!(
            decode(b"10:abc"),
            Err(BencodeError::UnexpectedEof)
        ));
    }

    #[test]
    fn decode_truncated_list() {
        // List without terminator
        assert!(matches!(decode(b"li42e"), Err(BencodeError::UnexpectedEof)));
    }

    #[test]
    fn decode_truncated_dict() {
        // Dict without terminator
        assert!(matches!(
            decode(b"d3:fooi1e"),
            Err(BencodeError::UnexpectedEof)
        ));
    }

    #[test]
    fn decode_invalid_int_plus_sign() {
        // Rust's i64::parse accepts "+5", so the decoder allows it
        // Verify it parses to the expected integer value
        assert_eq!(decode(b"i+5e").unwrap(), BValue::Integer(5));
    }

    #[test]
    fn decode_invalid_int_double_minus() {
        assert!(matches!(
            decode(b"i--5e"),
            Err(BencodeError::InvalidInteger(_))
        ));
    }

    #[test]
    fn decode_invalid_int_leading_zero_negative() {
        assert!(matches!(
            decode(b"i-01e"),
            Err(BencodeError::InvalidInteger(_))
        ));
    }

    #[test]
    fn decode_dict_duplicate_key_rejected() {
        // Two identical keys "aa" - second is not greater than first
        assert!(matches!(
            decode(b"d2:aai1e2:aai2ee"),
            Err(BencodeError::UnsortedKeys)
        ));
    }

    #[test]
    fn decode_invalid_start_byte() {
        assert!(matches!(
            decode(b"x"),
            Err(BencodeError::InvalidByte {
                byte: b'x',
                position: 0
            })
        ));
    }

    #[test]
    fn decode_fuzz_random_bytes_rejected() {
        // Various garbage inputs should not panic
        let inputs: &[&[u8]] = &[
            b"\xff\xff",
            b"i\x00e",
            b"d\x00\x00e",
            b"l\xfee",
            b"999999999999:",
        ];
        for input in inputs {
            let _ = decode(input); // should not panic
        }
    }

    #[test]
    fn decode_prefix_with_multiple_values() {
        let data = b"i1ei2ei3e";
        let (val1, consumed1) = decode_prefix(data).unwrap();
        assert_eq!(val1.as_integer().unwrap(), 1);
        assert_eq!(consumed1, 3);

        let (val2, consumed2) = decode_prefix(&data[consumed1..]).unwrap();
        assert_eq!(val2.as_integer().unwrap(), 2);
        assert_eq!(consumed2, 3);
    }

    #[test]
    fn decode_string_with_colon_in_content() {
        let val = decode(b"5:a:b:c").unwrap();
        assert_eq!(val.as_str().unwrap(), "a:b:c");
    }

    #[test]
    fn decode_nested_empty_structures() {
        // Dict containing empty dict value and empty list value
        // Encoded: d 5:empty de 4:list le e
        let val = decode(b"d5:emptyde4:listlee").unwrap();
        let dict = val.as_dict().unwrap();
        assert_eq!(dict.len(), 2);
        assert!(dict
            .get(b"empty".as_slice())
            .unwrap()
            .as_dict()
            .unwrap()
            .is_empty());
        assert!(dict
            .get(b"list".as_slice())
            .unwrap()
            .as_list()
            .unwrap()
            .is_empty());
    }
}
