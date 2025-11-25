//! Core Nostr event type
//!
//! This module defines the canonical in-memory representation of a Nostr event
//! that all serializers convert to/from.

use serde::{Deserialize, Serialize};

/// A Nostr event as defined in NIP-01
///
/// This struct stores cryptographic fields as raw bytes internally for efficiency,
/// but can serialize to/from hex strings for JSON compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NostrEvent {
    /// 32-byte event ID (SHA-256 hash)
    pub id: [u8; 32],

    /// 32-byte public key
    pub pubkey: [u8; 32],

    /// Unix timestamp in seconds
    pub created_at: i64,

    /// Event kind (0-65535)
    pub kind: u32,

    /// Array of tags, each tag is an array of strings
    pub tags: Vec<Vec<String>>,

    /// Event content (arbitrary string)
    pub content: String,

    /// 64-byte Schnorr signature
    pub sig: [u8; 64],
}

impl NostrEvent {
    /// Create a new NostrEvent from hex-encoded strings
    pub fn from_hex(
        id: &str,
        pubkey: &str,
        created_at: i64,
        kind: u32,
        tags: Vec<Vec<String>>,
        content: String,
        sig: &str,
    ) -> Result<Self, hex::FromHexError> {
        let id_bytes = hex::decode(id)?;
        let pubkey_bytes = hex::decode(pubkey)?;
        let sig_bytes = hex::decode(sig)?;

        Ok(Self {
            id: id_bytes
                .try_into()
                .map_err(|_| hex::FromHexError::InvalidStringLength)?,
            pubkey: pubkey_bytes
                .try_into()
                .map_err(|_| hex::FromHexError::InvalidStringLength)?,
            created_at,
            kind,
            tags,
            content,
            sig: sig_bytes
                .try_into()
                .map_err(|_| hex::FromHexError::InvalidStringLength)?,
        })
    }

    /// Get the event ID as a hex string
    pub fn id_hex(&self) -> String {
        hex::encode(self.id)
    }

    /// Get the pubkey as a hex string
    pub fn pubkey_hex(&self) -> String {
        hex::encode(self.pubkey)
    }

    /// Get the signature as a hex string
    pub fn sig_hex(&self) -> String {
        hex::encode(self.sig)
    }

    /// Calculate the total number of tags
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /// Calculate approximate JSON size (for categorization)
    pub fn estimated_json_size(&self) -> usize {
        // Base structure overhead
        let base = 100; // {"id":"","pubkey":"","created_at":,"kind":,"tags":[],"content":"","sig":""}

        // Hex-encoded fields
        let id_size = 64;
        let pubkey_size = 64;
        let sig_size = 128;

        // Timestamp and kind (variable, estimate)
        let timestamp_size = 10;
        let kind_size = 5;

        // Content (with some escaping overhead estimate)
        let content_size = self.content.len() + self.content.len() / 10;

        // Tags
        let tags_size: usize = self
            .tags
            .iter()
            .map(|tag| {
                // ["tag_name", "value1", ...] with quotes and commas
                4 + tag.iter().map(|s| s.len() + 3).sum::<usize>()
            })
            .sum();

        base + id_size
            + pubkey_size
            + sig_size
            + timestamp_size
            + kind_size
            + content_size
            + tags_size
    }

    /// Categorize event by size
    pub fn size_category(&self) -> SizeCategory {
        let size = self.estimated_json_size();
        match size {
            0..=500 => SizeCategory::Tiny,
            501..=2000 => SizeCategory::Small,
            2001..=10000 => SizeCategory::Medium,
            10001..=100000 => SizeCategory::Large,
            _ => SizeCategory::Huge,
        }
    }

    /// Categorize event by tag count
    pub fn tag_category(&self) -> TagCategory {
        match self.tag_count() {
            0 => TagCategory::None,
            1..=5 => TagCategory::Few,
            6..=20 => TagCategory::Moderate,
            21..=100 => TagCategory::Many,
            _ => TagCategory::Massive,
        }
    }
}

/// Size category for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeCategory {
    Tiny,   // < 500 bytes
    Small,  // 500B - 2KB
    Medium, // 2KB - 10KB
    Large,  // 10KB - 100KB
    Huge,   // > 100KB
}

impl std::fmt::Display for SizeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SizeCategory::Tiny => write!(f, "tiny (<500B)"),
            SizeCategory::Small => write!(f, "small (500B-2KB)"),
            SizeCategory::Medium => write!(f, "medium (2KB-10KB)"),
            SizeCategory::Large => write!(f, "large (10KB-100KB)"),
            SizeCategory::Huge => write!(f, "huge (>100KB)"),
        }
    }
}

/// Tag count category for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagCategory {
    None,     // 0 tags
    Few,      // 1-5 tags
    Moderate, // 6-20 tags
    Many,     // 21-100 tags
    Massive,  // 100+ tags
}

impl std::fmt::Display for TagCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagCategory::None => write!(f, "none (0)"),
            TagCategory::Few => write!(f, "few (1-5)"),
            TagCategory::Moderate => write!(f, "moderate (6-20)"),
            TagCategory::Many => write!(f, "many (21-100)"),
            TagCategory::Massive => write!(f, "massive (100+)"),
        }
    }
}

/// JSON-compatible representation for serde
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrEventJson {
    pub id: String,
    pub pubkey: String,
    pub created_at: i64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl From<&NostrEvent> for NostrEventJson {
    fn from(event: &NostrEvent) -> Self {
        Self {
            id: event.id_hex(),
            pubkey: event.pubkey_hex(),
            created_at: event.created_at,
            kind: event.kind,
            tags: event.tags.clone(),
            content: event.content.clone(),
            sig: event.sig_hex(),
        }
    }
}

impl TryFrom<NostrEventJson> for NostrEvent {
    type Error = hex::FromHexError;

    fn try_from(json: NostrEventJson) -> Result<Self, Self::Error> {
        NostrEvent::from_hex(
            &json.id,
            &json.pubkey,
            json.created_at,
            json.kind,
            json.tags,
            json.content,
            &json.sig,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> NostrEvent {
        NostrEvent {
            id: [0u8; 32],
            pubkey: [1u8; 32],
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                vec!["p".to_string(), "abc123".to_string()],
                vec![
                    "e".to_string(),
                    "def456".to_string(),
                    "wss://relay.example.com".to_string(),
                ],
            ],
            content: "Hello, Nostr!".to_string(),
            sig: [2u8; 64],
        }
    }

    #[test]
    fn test_hex_roundtrip() {
        let event = sample_event();
        let json = NostrEventJson::from(&event);
        let back: NostrEvent = json.try_into().unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_size_category() {
        let event = sample_event();
        assert_eq!(event.size_category(), SizeCategory::Tiny);
    }

    #[test]
    fn test_tag_category() {
        let event = sample_event();
        assert_eq!(event.tag_category(), TagCategory::Few);
    }
}
