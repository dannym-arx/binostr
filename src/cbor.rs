//! CBOR serialization variants
//!
//! Three CBOR encoding strategies:
//! 1. Schemaless - JSON-like with string field names
//! 2. Packed Array - positional encoding, smallest size
//! 3. Integer-keyed Map - balance of size and extensibility

use ciborium::value::Value;
use serde::{Deserialize, Serialize};

use crate::event::NostrEvent;

// ============================================
// Variant 1: Schemaless (JSON-like)
// ============================================

/// CBOR schemaless format - uses string field names like JSON
/// but stores binary data as bytes instead of hex
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CborSchemaless {
    #[serde(with = "serde_bytes")]
    pub id: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub pubkey: Vec<u8>,
    pub created_at: i64,
    pub kind: u16,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    #[serde(with = "serde_bytes")]
    pub sig: Vec<u8>,
}

impl From<&NostrEvent> for CborSchemaless {
    fn from(event: &NostrEvent) -> Self {
        Self {
            id: event.id.to_vec(),
            pubkey: event.pubkey.to_vec(),
            created_at: event.created_at,
            kind: event.kind,
            tags: event.tags.clone(),
            content: event.content.clone(),
            sig: event.sig.to_vec(),
        }
    }
}

impl TryFrom<CborSchemaless> for NostrEvent {
    type Error = CborError;

    fn try_from(cbor: CborSchemaless) -> Result<Self, Self::Error> {
        Ok(Self {
            id: cbor
                .id
                .try_into()
                .map_err(|_| CborError::InvalidLength("id"))?,
            pubkey: cbor
                .pubkey
                .try_into()
                .map_err(|_| CborError::InvalidLength("pubkey"))?,
            created_at: cbor.created_at,
            kind: cbor.kind,
            tags: cbor.tags,
            content: cbor.content,
            sig: cbor
                .sig
                .try_into()
                .map_err(|_| CborError::InvalidLength("sig"))?,
        })
    }
}

pub mod schemaless {
    use super::*;

    pub fn serialize(event: &NostrEvent) -> Vec<u8> {
        let cbor = CborSchemaless::from(event);
        let mut buf = Vec::new();
        ciborium::into_writer(&cbor, &mut buf).expect("CBOR serialization should not fail");
        buf
    }

    pub fn deserialize(data: &[u8]) -> Result<NostrEvent, CborError> {
        let cbor: CborSchemaless = ciborium::from_reader(data)?;
        NostrEvent::try_from(cbor)
    }

    pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
        let cbor_events: Vec<CborSchemaless> = events.iter().map(CborSchemaless::from).collect();
        let mut buf = Vec::new();
        ciborium::into_writer(&cbor_events, &mut buf).expect("CBOR serialization should not fail");
        buf
    }

    pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, CborError> {
        let cbor_events: Vec<CborSchemaless> = ciborium::from_reader(data)?;
        cbor_events.into_iter().map(NostrEvent::try_from).collect()
    }
}

// ============================================
// Variant 2: Packed Array (positional)
// ============================================

// Packed format: [id, pubkey, created_at, kind, tags, content, sig]
// No field names = smallest size

pub mod packed {
    use super::*;

    pub fn serialize(event: &NostrEvent) -> Vec<u8> {
        let value = Value::Array(vec![
            Value::Bytes(event.id.to_vec()),
            Value::Bytes(event.pubkey.to_vec()),
            Value::Integer(event.created_at.into()),
            Value::Integer(event.kind.into()),
            tags_to_value(&event.tags),
            Value::Text(event.content.clone()),
            Value::Bytes(event.sig.to_vec()),
        ]);

        let mut buf = Vec::new();
        ciborium::into_writer(&value, &mut buf).expect("CBOR serialization should not fail");
        buf
    }

    pub fn deserialize(data: &[u8]) -> Result<NostrEvent, CborError> {
        let value: Value = ciborium::from_reader(data)?;

        let arr = value.as_array().ok_or(CborError::ExpectedArray)?;
        if arr.len() != 7 {
            return Err(CborError::InvalidLength("event array"));
        }

        let id = extract_bytes(&arr[0], "id")?;
        let pubkey = extract_bytes(&arr[1], "pubkey")?;
        let created_at = extract_i64(&arr[2], "created_at")?;
        let kind = extract_u16(&arr[3], "kind")?;
        let tags = extract_tags(&arr[4])?;
        let content = extract_string(&arr[5], "content")?;
        let sig = extract_bytes(&arr[6], "sig")?;

        Ok(NostrEvent {
            id: id.try_into().map_err(|_| CborError::InvalidLength("id"))?,
            pubkey: pubkey
                .try_into()
                .map_err(|_| CborError::InvalidLength("pubkey"))?,
            created_at,
            kind,
            tags,
            content,
            sig: sig
                .try_into()
                .map_err(|_| CborError::InvalidLength("sig"))?,
        })
    }

    pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
        let values: Vec<Value> = events
            .iter()
            .map(|e| {
                Value::Array(vec![
                    Value::Bytes(e.id.to_vec()),
                    Value::Bytes(e.pubkey.to_vec()),
                    Value::Integer(e.created_at.into()),
                    Value::Integer(e.kind.into()),
                    tags_to_value(&e.tags),
                    Value::Text(e.content.clone()),
                    Value::Bytes(e.sig.to_vec()),
                ])
            })
            .collect();

        let mut buf = Vec::new();
        ciborium::into_writer(&Value::Array(values), &mut buf)
            .expect("CBOR serialization should not fail");
        buf
    }

    pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, CborError> {
        let value: Value = ciborium::from_reader(data)?;
        let arr = value.as_array().ok_or(CborError::ExpectedArray)?;

        arr.iter()
            .map(|v| {
                let arr = v.as_array().ok_or(CborError::ExpectedArray)?;
                if arr.len() != 7 {
                    return Err(CborError::InvalidLength("event array"));
                }

                Ok(NostrEvent {
                    id: extract_bytes(&arr[0], "id")?
                        .try_into()
                        .map_err(|_| CborError::InvalidLength("id"))?,
                    pubkey: extract_bytes(&arr[1], "pubkey")?
                        .try_into()
                        .map_err(|_| CborError::InvalidLength("pubkey"))?,
                    created_at: extract_i64(&arr[2], "created_at")?,
                    kind: extract_u16(&arr[3], "kind")?,
                    tags: extract_tags(&arr[4])?,
                    content: extract_string(&arr[5], "content")?,
                    sig: extract_bytes(&arr[6], "sig")?
                        .try_into()
                        .map_err(|_| CborError::InvalidLength("sig"))?,
                })
            })
            .collect()
    }
}

// ============================================
// Variant 3: Integer-keyed Map
// ============================================

// Integer keys: {0: id, 1: pubkey, 2: created_at, 3: kind, 4: tags, 5: content, 6: sig}

pub mod intkey {
    use super::*;

    pub fn serialize(event: &NostrEvent) -> Vec<u8> {
        let value = Value::Map(vec![
            (Value::Integer(0.into()), Value::Bytes(event.id.to_vec())),
            (
                Value::Integer(1.into()),
                Value::Bytes(event.pubkey.to_vec()),
            ),
            (
                Value::Integer(2.into()),
                Value::Integer(event.created_at.into()),
            ),
            (Value::Integer(3.into()), Value::Integer(event.kind.into())),
            (Value::Integer(4.into()), tags_to_value(&event.tags)),
            (Value::Integer(5.into()), Value::Text(event.content.clone())),
            (Value::Integer(6.into()), Value::Bytes(event.sig.to_vec())),
        ]);

        let mut buf = Vec::new();
        ciborium::into_writer(&value, &mut buf).expect("CBOR serialization should not fail");
        buf
    }

    pub fn deserialize(data: &[u8]) -> Result<NostrEvent, CborError> {
        let value: Value = ciborium::from_reader(data)?;

        let map = value.as_map().ok_or(CborError::ExpectedMap)?;

        let mut id = None;
        let mut pubkey = None;
        let mut created_at = None;
        let mut kind = None;
        let mut tags = None;
        let mut content = None;
        let mut sig = None;

        for (k, v) in map {
            let key = k.as_integer().ok_or(CborError::ExpectedInteger("key"))?;
            let key: i128 = key.into();

            match key {
                0 => id = Some(extract_bytes(v, "id")?),
                1 => pubkey = Some(extract_bytes(v, "pubkey")?),
                2 => created_at = Some(extract_i64(v, "created_at")?),
                3 => kind = Some(extract_u16(v, "kind")?),
                4 => tags = Some(extract_tags(v)?),
                5 => content = Some(extract_string(v, "content")?),
                6 => sig = Some(extract_bytes(v, "sig")?),
                _ => {} // Ignore unknown keys for forward compatibility
            }
        }

        Ok(NostrEvent {
            id: id
                .ok_or(CborError::MissingField("id"))?
                .try_into()
                .map_err(|_| CborError::InvalidLength("id"))?,
            pubkey: pubkey
                .ok_or(CborError::MissingField("pubkey"))?
                .try_into()
                .map_err(|_| CborError::InvalidLength("pubkey"))?,
            created_at: created_at.ok_or(CborError::MissingField("created_at"))?,
            kind: kind.ok_or(CborError::MissingField("kind"))?,
            tags: tags.ok_or(CborError::MissingField("tags"))?,
            content: content.ok_or(CborError::MissingField("content"))?,
            sig: sig
                .ok_or(CborError::MissingField("sig"))?
                .try_into()
                .map_err(|_| CborError::InvalidLength("sig"))?,
        })
    }

    pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
        let values: Vec<Value> = events
            .iter()
            .map(|e| {
                Value::Map(vec![
                    (Value::Integer(0.into()), Value::Bytes(e.id.to_vec())),
                    (Value::Integer(1.into()), Value::Bytes(e.pubkey.to_vec())),
                    (
                        Value::Integer(2.into()),
                        Value::Integer(e.created_at.into()),
                    ),
                    (Value::Integer(3.into()), Value::Integer(e.kind.into())),
                    (Value::Integer(4.into()), tags_to_value(&e.tags)),
                    (Value::Integer(5.into()), Value::Text(e.content.clone())),
                    (Value::Integer(6.into()), Value::Bytes(e.sig.to_vec())),
                ])
            })
            .collect();

        let mut buf = Vec::new();
        ciborium::into_writer(&Value::Array(values), &mut buf)
            .expect("CBOR serialization should not fail");
        buf
    }

    pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, CborError> {
        let value: Value = ciborium::from_reader(data)?;
        let arr = value.as_array().ok_or(CborError::ExpectedArray)?;

        arr.iter()
            .map(|v| {
                let map = v.as_map().ok_or(CborError::ExpectedMap)?;

                let mut id = None;
                let mut pubkey = None;
                let mut created_at = None;
                let mut kind = None;
                let mut tags = None;
                let mut content = None;
                let mut sig = None;

                for (k, val) in map {
                    let key = k.as_integer().ok_or(CborError::ExpectedInteger("key"))?;
                    let key: i128 = key.into();

                    match key {
                        0 => id = Some(extract_bytes(val, "id")?),
                        1 => pubkey = Some(extract_bytes(val, "pubkey")?),
                        2 => created_at = Some(extract_i64(val, "created_at")?),
                        3 => kind = Some(extract_u16(val, "kind")?),
                        4 => tags = Some(extract_tags(val)?),
                        5 => content = Some(extract_string(val, "content")?),
                        6 => sig = Some(extract_bytes(val, "sig")?),
                        _ => {}
                    }
                }

                Ok(NostrEvent {
                    id: id
                        .ok_or(CborError::MissingField("id"))?
                        .try_into()
                        .map_err(|_| CborError::InvalidLength("id"))?,
                    pubkey: pubkey
                        .ok_or(CborError::MissingField("pubkey"))?
                        .try_into()
                        .map_err(|_| CborError::InvalidLength("pubkey"))?,
                    created_at: created_at.ok_or(CborError::MissingField("created_at"))?,
                    kind: kind.ok_or(CborError::MissingField("kind"))?,
                    tags: tags.ok_or(CborError::MissingField("tags"))?,
                    content: content.ok_or(CborError::MissingField("content"))?,
                    sig: sig
                        .ok_or(CborError::MissingField("sig"))?
                        .try_into()
                        .map_err(|_| CborError::InvalidLength("sig"))?,
                })
            })
            .collect()
    }
}

// ============================================
// Helper functions
// ============================================

/// Check if a string contains only hex characters (0-9, a-f, A-F)
fn is_hex_string(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Encode a tag value optimally: if it's hex, decode to bytes (50% size reduction),
/// otherwise store as text
fn encode_tag_value_cbor(value: &str) -> Value {
    if is_hex_string(value) && value.len().is_multiple_of(2) {
        // Try to decode as hex - if successful, store as bytes
        if let Ok(bytes) = hex::decode(value) {
            return Value::Bytes(bytes);
        }
    }
    // Store as text
    Value::Text(value.to_string())
}

/// Decode a tag value from CBOR Value back to string
fn decode_tag_value_cbor(value: &Value) -> Result<String, CborError> {
    match value {
        Value::Bytes(bytes) => {
            // Decode hex bytes back to hex string
            Ok(hex::encode(bytes))
        }
        Value::Text(text) => Ok(text.clone()),
        _ => Err(CborError::ExpectedString("tag value")),
    }
}

fn tags_to_value(tags: &[Vec<String>]) -> Value {
    Value::Array(
        tags.iter()
            .map(|tag| Value::Array(tag.iter().map(|v| encode_tag_value_cbor(v)).collect()))
            .collect(),
    )
}

fn extract_bytes(value: &Value, field: &'static str) -> Result<Vec<u8>, CborError> {
    value
        .as_bytes()
        .map(|b| b.to_vec())
        .ok_or(CborError::ExpectedBytes(field))
}

fn extract_i64(value: &Value, field: &'static str) -> Result<i64, CborError> {
    value
        .as_integer()
        .and_then(|i| {
            let i: i128 = i.into();
            i64::try_from(i).ok()
        })
        .ok_or(CborError::ExpectedInteger(field))
}

fn extract_u16(value: &Value, field: &'static str) -> Result<u16, CborError> {
    value
        .as_integer()
        .and_then(|i| {
            let i: i128 = i.into();
            u16::try_from(i).ok()
        })
        .ok_or(CborError::ExpectedInteger(field))
}

fn extract_string(value: &Value, field: &'static str) -> Result<String, CborError> {
    value
        .as_text()
        .map(|s| s.to_string())
        .ok_or(CborError::ExpectedString(field))
}

fn extract_tags(value: &Value) -> Result<Vec<Vec<String>>, CborError> {
    let arr = value.as_array().ok_or(CborError::ExpectedArray)?;

    arr.iter()
        .map(|tag_value| {
            let tag_arr = tag_value.as_array().ok_or(CborError::ExpectedArray)?;
            tag_arr.iter().map(decode_tag_value_cbor).collect()
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum CborError {
    #[error("CBOR error: {0}")]
    Ciborium(#[from] ciborium::de::Error<std::io::Error>),

    #[error("Expected array")]
    ExpectedArray,

    #[error("Expected map")]
    ExpectedMap,

    #[error("Expected bytes for field: {0}")]
    ExpectedBytes(&'static str),

    #[error("Expected integer for field: {0}")]
    ExpectedInteger(&'static str),

    #[error("Expected string for field: {0}")]
    ExpectedString(&'static str),

    #[error("Invalid length for field: {0}")]
    InvalidLength(&'static str),

    #[error("Missing field: {0}")]
    MissingField(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> NostrEvent {
        NostrEvent {
            id: [0xab; 32],
            pubkey: [0xcd; 32],
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                vec!["p".to_string(), "abc123".to_string()],
                vec!["e".to_string(), "def456".to_string()],
            ],
            content: "Hello, Nostr!".to_string(),
            sig: [0xef; 64],
        }
    }

    #[test]
    fn test_schemaless_roundtrip() {
        let event = sample_event();
        let bytes = schemaless::serialize(&event);
        let back = schemaless::deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_packed_roundtrip() {
        let event = sample_event();
        let bytes = packed::serialize(&event);
        let back = packed::deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_intkey_roundtrip() {
        let event = sample_event();
        let bytes = intkey::serialize(&event);
        let back = intkey::deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_size_comparison() {
        let event = sample_event();

        let json_size = crate::json::serialize(&event).len();
        let schemaless_size = schemaless::serialize(&event).len();
        let packed_size = packed::serialize(&event).len();
        let intkey_size = intkey::serialize(&event).len();

        // Packed should be smallest, schemaless should be smaller than JSON
        println!("JSON: {} bytes", json_size);
        println!("CBOR Schemaless: {} bytes", schemaless_size);
        println!("CBOR Packed: {} bytes", packed_size);
        println!("CBOR IntKey: {} bytes", intkey_size);

        assert!(packed_size < schemaless_size);
        assert!(schemaless_size < json_size);
    }

    #[test]
    fn test_batch_roundtrip() {
        let events = vec![sample_event(), sample_event()];

        // Schemaless batch
        let bytes = schemaless::serialize_batch(&events);
        let back = schemaless::deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);

        // Packed batch
        let bytes = packed::serialize_batch(&events);
        let back = packed::deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);

        // IntKey batch
        let bytes = intkey::serialize_batch(&events);
        let back = intkey::deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);
    }
}
