use std::collections::BTreeMap;

/// A BEncoded value.
///
/// BEncoding supports four value types:
/// - Byte strings (arbitrary binary data)
/// - Integers (signed 64-bit)
/// - Lists (ordered sequences of values)
/// - Dictionaries (sorted key-value maps with byte-string keys)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BValue {
    /// A byte string. May contain arbitrary binary data.
    String(Vec<u8>),
    /// A signed 64-bit integer.
    Integer(i64),
    /// An ordered list of BEncoded values.
    List(Vec<BValue>),
    /// A dictionary mapping byte-string keys to BEncoded values.
    /// Keys are stored in a `BTreeMap`, which maintains sorted order.
    Dict(BTreeMap<Vec<u8>, BValue>),
}

impl BValue {
    /// Returns the value as a byte slice if it is a `String`, or `None` otherwise.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            BValue::String(b) => Some(b),
            _ => None,
        }
    }

    /// Returns the value as a UTF-8 string if it is a `String` with valid UTF-8, or `None`.
    pub fn as_str(&self) -> Option<&str> {
        self.as_bytes().and_then(|b| std::str::from_utf8(b).ok())
    }

    /// Returns the integer value if this is an `Integer`, or `None` otherwise.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            BValue::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns a reference to the list if this is a `List`, or `None` otherwise.
    pub fn as_list(&self) -> Option<&[BValue]> {
        match self {
            BValue::List(l) => Some(l),
            _ => None,
        }
    }

    /// Returns a reference to the dictionary if this is a `Dict`, or `None` otherwise.
    pub fn as_dict(&self) -> Option<&BTreeMap<Vec<u8>, BValue>> {
        match self {
            BValue::Dict(d) => Some(d),
            _ => None,
        }
    }

    /// Looks up a key in this value if it is a dictionary.
    /// The key is treated as UTF-8 bytes.
    pub fn dict_get(&self, key: &str) -> Option<&BValue> {
        self.as_dict().and_then(|d| d.get(key.as_bytes()))
    }
}
