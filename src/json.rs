//! JSON serialization (baseline)
//!
//! Standard NIP-01 JSON format using serde_json.

use crate::event::{NostrEvent, NostrEventJson};

/// Serialize a NostrEvent to JSON bytes
pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    let json_event = NostrEventJson::from(event);
    serde_json::to_vec(&json_event).expect("JSON serialization should not fail")
}

/// Serialize a NostrEvent to a JSON string
pub fn serialize_string(event: &NostrEvent) -> String {
    let json_event = NostrEventJson::from(event);
    serde_json::to_string(&json_event).expect("JSON serialization should not fail")
}

/// Serialize a NostrEvent to compact JSON bytes (no pretty printing)
pub fn serialize_compact(event: &NostrEvent) -> Vec<u8> {
    // serde_json::to_vec already produces compact JSON
    serialize(event)
}

/// Deserialize a NostrEvent from JSON bytes
pub fn deserialize(data: &[u8]) -> Result<NostrEvent, JsonError> {
    let json_event: NostrEventJson = serde_json::from_slice(data)?;
    let event = NostrEvent::try_from(json_event)?;
    Ok(event)
}

/// Deserialize a NostrEvent from a JSON string
pub fn deserialize_str(data: &str) -> Result<NostrEvent, JsonError> {
    let json_event: NostrEventJson = serde_json::from_str(data)?;
    let event = NostrEvent::try_from(json_event)?;
    Ok(event)
}

/// Serialize a batch of events to JSON array
pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    let json_events: Vec<NostrEventJson> = events.iter().map(NostrEventJson::from).collect();
    serde_json::to_vec(&json_events).expect("JSON serialization should not fail")
}

/// Deserialize a batch of events from JSON array
pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, JsonError> {
    let json_events: Vec<NostrEventJson> = serde_json::from_slice(data)?;
    json_events
        .into_iter()
        .map(NostrEvent::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(JsonError::Hex)
}

#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
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
    fn test_roundtrip() {
        let event = sample_event();
        let bytes = serialize(&event);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_batch_roundtrip() {
        let events = vec![sample_event(), sample_event()];
        let bytes = serialize_batch(&events);
        let back = deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);
    }

    #[test]
    fn test_json_format() {
        let event = sample_event();
        let json = serialize_string(&event);

        // Verify it's valid JSON with expected fields
        assert!(json.contains("\"id\":"));
        assert!(json.contains("\"pubkey\":"));
        assert!(json.contains("\"created_at\":"));
        assert!(json.contains("\"kind\":"));
        assert!(json.contains("\"tags\":"));
        assert!(json.contains("\"content\":"));
        assert!(json.contains("\"sig\":"));
    }
}
