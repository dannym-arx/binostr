//! Cap'n Proto serialization
//!
//! Cap'n Proto is extremely fast because there's no encoding/decoding step -
//! the in-memory representation IS the wire format.
//! See: https://capnproto.org/

use capnp::message::{Builder, ReaderOptions};
use capnp::serialize;

use crate::event::NostrEvent;

// Include the generated Cap'n Proto code
pub mod nostr_capnp {
    include!(concat!(env!("OUT_DIR"), "/nostr_capnp.rs"));
}

use nostr_capnp::nostr_event;

/// Serialize a NostrEvent to Cap'n Proto format
pub fn serialize_event(event: &NostrEvent) -> Vec<u8> {
    let mut message = Builder::new_default();

    {
        let mut builder = message.init_root::<nostr_event::Builder>();
        builder.set_id(&event.id);
        builder.set_pubkey(&event.pubkey);
        builder.set_created_at(event.created_at);
        builder.set_kind(event.kind);
        builder.set_content(&event.content);
        builder.set_sig(&event.sig);

        // Build tags
        let mut tags_builder = builder.init_tags(event.tags.len() as u32);
        for (i, tag) in event.tags.iter().enumerate() {
            let tag_builder = tags_builder.reborrow().get(i as u32);
            let mut values_builder = tag_builder.init_values(tag.len() as u32);
            for (j, value) in tag.iter().enumerate() {
                values_builder.set(j as u32, value);
            }
        }
    }

    let mut buf = Vec::new();
    serialize::write_message(&mut buf, &message).expect("Cap'n Proto serialization failed");
    buf
}

/// Deserialize a NostrEvent from Cap'n Proto format
pub fn deserialize_event(data: &[u8]) -> Result<NostrEvent, CapnpError> {
    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let event_reader = reader.get_root::<nostr_event::Reader>()?;

    let id: [u8; 32] = event_reader
        .get_id()?
        .try_into()
        .map_err(|_| CapnpError::InvalidLength("id"))?;

    let pubkey: [u8; 32] = event_reader
        .get_pubkey()?
        .try_into()
        .map_err(|_| CapnpError::InvalidLength("pubkey"))?;

    let sig: [u8; 64] = event_reader
        .get_sig()?
        .try_into()
        .map_err(|_| CapnpError::InvalidLength("sig"))?;

    let tags_reader = event_reader.get_tags()?;
    let mut tags = Vec::with_capacity(tags_reader.len() as usize);

    for tag_reader in tags_reader.iter() {
        let values_reader = tag_reader.get_values()?;
        let mut values = Vec::with_capacity(values_reader.len() as usize);
        for value in values_reader.iter() {
            values.push(value?.to_string()?);
        }
        tags.push(values);
    }

    let content = event_reader.get_content()?.to_string()?;

    Ok(NostrEvent {
        id,
        pubkey,
        created_at: event_reader.get_created_at(),
        kind: event_reader.get_kind(),
        tags,
        content,
        sig,
    })
}

/// Serialize a batch of events to Cap'n Proto format
pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    use nostr_capnp::event_batch;

    let mut message = Builder::new_default();

    {
        let builder = message.init_root::<event_batch::Builder>();
        let mut events_builder = builder.init_events(events.len() as u32);

        for (i, event) in events.iter().enumerate() {
            let mut event_builder = events_builder.reborrow().get(i as u32);
            event_builder.set_id(&event.id);
            event_builder.set_pubkey(&event.pubkey);
            event_builder.set_created_at(event.created_at);
            event_builder.set_kind(event.kind);
            event_builder.set_content(&event.content);
            event_builder.set_sig(&event.sig);

            let mut tags_builder = event_builder.init_tags(event.tags.len() as u32);
            for (j, tag) in event.tags.iter().enumerate() {
                let tag_builder = tags_builder.reborrow().get(j as u32);
                let mut values_builder = tag_builder.init_values(tag.len() as u32);
                for (k, value) in tag.iter().enumerate() {
                    values_builder.set(k as u32, value);
                }
            }
        }
    }

    let mut buf = Vec::new();
    serialize::write_message(&mut buf, &message).expect("Cap'n Proto serialization failed");
    buf
}

/// Deserialize a batch of events from Cap'n Proto format
pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, CapnpError> {
    use nostr_capnp::event_batch;

    let reader = serialize::read_message(data, ReaderOptions::new())?;
    let batch_reader = reader.get_root::<event_batch::Reader>()?;
    let events_reader = batch_reader.get_events()?;

    let mut events = Vec::with_capacity(events_reader.len() as usize);

    for event_reader in events_reader.iter() {
        let id: [u8; 32] = event_reader
            .get_id()?
            .try_into()
            .map_err(|_| CapnpError::InvalidLength("id"))?;

        let pubkey: [u8; 32] = event_reader
            .get_pubkey()?
            .try_into()
            .map_err(|_| CapnpError::InvalidLength("pubkey"))?;

        let sig: [u8; 64] = event_reader
            .get_sig()?
            .try_into()
            .map_err(|_| CapnpError::InvalidLength("sig"))?;

        let tags_reader = event_reader.get_tags()?;
        let mut tags = Vec::with_capacity(tags_reader.len() as usize);

        for tag_reader in tags_reader.iter() {
            let values_reader = tag_reader.get_values()?;
            let mut values = Vec::with_capacity(values_reader.len() as usize);
            for value in values_reader.iter() {
                values.push(value?.to_string()?);
            }
            tags.push(values);
        }

        let content = event_reader.get_content()?.to_string()?;

        events.push(NostrEvent {
            id,
            pubkey,
            created_at: event_reader.get_created_at(),
            kind: event_reader.get_kind(),
            tags,
            content,
            sig,
        });
    }

    Ok(events)
}

#[derive(Debug, thiserror::Error)]
pub enum CapnpError {
    #[error("Cap'n Proto error: {0}")]
    Capnp(#[from] capnp::Error),

    #[error("Cap'n Proto not in schema: {0}")]
    NotInSchema(#[from] capnp::NotInSchema),

    #[error("Invalid length for field: {0}")]
    InvalidLength(&'static str),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
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
        let bytes = serialize_event(&event);
        let back = deserialize_event(&bytes).unwrap();
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
    fn test_size_comparison() {
        let event = sample_event();

        let capnp_size = serialize_event(&event).len();
        let json_size = crate::json::serialize(&event).len();

        println!("Cap'n Proto: {} bytes", capnp_size);
        println!("JSON: {} bytes", json_size);

        // Cap'n Proto may be larger due to alignment/padding but is much faster
    }
}
