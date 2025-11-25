//! Common utilities for benchmarks

use binostr::{EventSampler, NostrEvent};
use criterion::Criterion;
use std::time::Duration;

/// Create a fast Criterion configuration for quicker iterations
/// Use `cargo bench` for normal runs, `cargo bench -- --quick` isn't needed with this
#[allow(dead_code)]
pub fn fast_criterion() -> Criterion {
    Criterion::default()
        .sample_size(30) // Default: 100
        .measurement_time(Duration::from_secs(2)) // Default: 5s
        .warm_up_time(Duration::from_millis(500)) // Default: 3s
        .confidence_level(0.90) // Default: 0.95
}

/// Default data directory
pub const DATA_DIR: &str = "data";

/// Load a sample of events for benchmarking
#[allow(dead_code)]
pub fn load_sample(size: usize) -> Vec<NostrEvent> {
    let mut sampler = match EventSampler::from_directory(DATA_DIR, size * 2) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Could not load events from {}: {}", DATA_DIR, e);
            eprintln!("Using synthetic test events instead.");
            return generate_synthetic_events(size);
        }
    };

    if sampler.len() < size {
        eprintln!(
            "Warning: Only loaded {} events, requested {}",
            sampler.len(),
            size
        );
    }

    sampler.random_sample(size).into_iter().cloned().collect()
}

/// Load events filtered by kind
#[allow(dead_code)]
pub fn load_by_kind(kind: u16, size: usize) -> Vec<NostrEvent> {
    let mut sampler = match EventSampler::from_directory(DATA_DIR, size * 10) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Could not load events: {}", e);
            return generate_synthetic_events_kind(kind, size);
        }
    };

    let events = sampler.sample_kind(kind, size);
    if events.is_empty() {
        eprintln!("Warning: No events found for kind {}", kind);
        return generate_synthetic_events_kind(kind, size);
    }

    events.into_iter().cloned().collect()
}

/// Generate synthetic events for testing when data files aren't available
#[allow(dead_code)]
pub fn generate_synthetic_events(count: usize) -> Vec<NostrEvent> {
    (0..count)
        .map(|i| NostrEvent {
            id: {
                let mut arr = [0u8; 32];
                arr[0..8].copy_from_slice(&(i as u64).to_le_bytes());
                arr
            },
            pubkey: [0xab; 32],
            created_at: 1700000000 + i as i64,
            kind: 1,
            tags: vec![
                vec!["p".to_string(), hex::encode([0xcd; 32])],
                vec!["e".to_string(), hex::encode([0xef; 32])],
            ],
            content: format!("This is test event number {}. Hello Nostr!", i),
            sig: [0x12; 64],
        })
        .collect()
}

/// Generate synthetic events of a specific kind
#[allow(dead_code)]
pub fn generate_synthetic_events_kind(kind: u16, count: usize) -> Vec<NostrEvent> {
    (0..count)
        .map(|i| {
            let (tags, content) = match kind {
                0 => {
                    // Profile metadata
                    (
                        vec![],
                        format!(
                            r#"{{"name":"user{}","about":"A test user","picture":"https://example.com/pic.jpg"}}"#,
                            i
                        ),
                    )
                }
                1 => {
                    // Short text note
                    (
                        vec![
                            vec!["p".to_string(), hex::encode([i as u8; 32])],
                            vec!["t".to_string(), "nostr".to_string()],
                        ],
                        format!("This is test note {}. #nostr", i),
                    )
                }
                3 => {
                    // Follow list (many p tags)
                    let tags: Vec<Vec<String>> = (0..100)
                        .map(|j| {
                            vec![
                                "p".to_string(),
                                hex::encode([(i + j) as u8; 32]),
                                "wss://relay.example.com".to_string(),
                            ]
                        })
                        .collect();
                    (tags, String::new())
                }
                7 => {
                    // Reaction
                    (
                        vec![
                            vec!["e".to_string(), hex::encode([i as u8; 32])],
                            vec!["p".to_string(), hex::encode([(i + 1) as u8; 32])],
                        ],
                        "🤙".to_string(),
                    )
                }
                30023 => {
                    // Long-form article
                    let content = format!(
                        "# Article {}\n\n{}\n\n## Section 1\n\n{}\n\n## Section 2\n\n{}",
                        i,
                        "Lorem ipsum ".repeat(50),
                        "Dolor sit amet ".repeat(100),
                        "Consectetur adipiscing ".repeat(100),
                    );
                    (
                        vec![
                            vec!["d".to_string(), format!("article-{}", i)],
                            vec!["title".to_string(), format!("Test Article {}", i)],
                            vec!["published_at".to_string(), "1700000000".to_string()],
                        ],
                        content,
                    )
                }
                _ => {
                    // Generic event
                    (vec![], format!("Kind {} event {}", kind, i))
                }
            };

            NostrEvent {
                id: {
                    let mut arr = [0u8; 32];
                    arr[0..8].copy_from_slice(&(i as u64).to_le_bytes());
                    arr
                },
                pubkey: [0xab; 32],
                created_at: 1700000000 + i as i64,
                kind,
                tags,
                content,
                sig: [0x12; 64],
            }
        })
        .collect()
}
