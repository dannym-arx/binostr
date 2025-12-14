//! Benchmarks by event kind

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

mod common;

use binostr::{capnp, cbor, json, notepack, proto, NostrEvent};

/// Benchmark a specific event kind
fn bench_kind(c: &mut Criterion, kind: u16, name: &str) {
    let events = common::load_by_kind(kind, 100);

    if events.is_empty() {
        eprintln!("No events for kind {}, skipping", kind);
        return;
    }

    let group_name = format!("kind_{}_{}", kind, name);
    let mut group = c.benchmark_group(&group_name);
    group.throughput(Throughput::Elements(events.len() as u64));

    // Serialize benchmarks
    group.bench_function("serialize/json", |b| {
        b.iter(|| {
            for event in &events {
                black_box(json::serialize(event));
            }
        })
    });

    group.bench_function("serialize/cbor_packed", |b| {
        b.iter(|| {
            for event in &events {
                black_box(cbor::packed::serialize(event));
            }
        })
    });

    group.bench_function("serialize/proto_binary", |b| {
        b.iter(|| {
            for event in &events {
                black_box(proto::binary::serialize(event));
            }
        })
    });

    group.bench_function("serialize/capnp", |b| {
        b.iter(|| {
            for event in &events {
                black_box(capnp::serialize_event(event));
            }
        })
    });

    group.bench_function("serialize/notepack", |b| {
        b.iter(|| {
            for event in &events {
                black_box(notepack::serialize(event));
            }
        })
    });

    // Pre-serialize for deserialization benchmarks
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let cbor_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let proto_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let notepack_data: Vec<_> = events.iter().map(notepack::serialize).collect();

    // Deserialize benchmarks
    group.bench_function("deserialize/json", |b| {
        b.iter(|| {
            for data in &json_data {
                black_box(json::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/cbor_packed", |b| {
        b.iter(|| {
            for data in &cbor_data {
                black_box(cbor::packed::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/proto_binary", |b| {
        b.iter(|| {
            for data in &proto_data {
                black_box(proto::binary::deserialize(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/capnp", |b| {
        b.iter(|| {
            for data in &capnp_data {
                black_box(capnp::deserialize_event(data).unwrap());
            }
        })
    });

    group.bench_function("deserialize/notepack", |b| {
        b.iter(|| {
            for data in &notepack_data {
                black_box(notepack::deserialize(data).unwrap());
            }
        })
    });

    group.finish();

    // Print size comparison for this kind
    print_size_comparison(&events, kind, name);
}

fn print_size_comparison(events: &[NostrEvent], kind: u16, name: &str) {
    if events.is_empty() {
        return;
    }

    let mut json_total = 0;
    let mut cbor_schemaless_total = 0;
    let mut cbor_packed_total = 0;
    let mut cbor_intkey_total = 0;
    let mut proto_string_total = 0;
    let mut proto_binary_total = 0;
    let mut capnp_total = 0;
    let mut notepack_total = 0;

    for event in events {
        json_total += json::serialize(event).len();
        cbor_schemaless_total += cbor::schemaless::serialize(event).len();
        cbor_packed_total += cbor::packed::serialize(event).len();
        cbor_intkey_total += cbor::intkey::serialize(event).len();
        proto_string_total += proto::string::serialize(event).len();
        proto_binary_total += proto::binary::serialize(event).len();
        capnp_total += capnp::serialize_event(event).len();
        notepack_total += notepack::serialize(event).len();
    }

    let n = events.len();
    println!("\n=== Kind {} ({}) - {} events ===", kind, name, n);
    println!("Average sizes:");
    println!("  JSON:           {:>6} bytes (100.0%)", json_total / n);
    println!(
        "  CBOR Schemaless:{:>6} bytes ({:>5.1}%)",
        cbor_schemaless_total / n,
        100.0 * cbor_schemaless_total as f64 / json_total as f64
    );
    println!(
        "  CBOR Packed:    {:>6} bytes ({:>5.1}%)",
        cbor_packed_total / n,
        100.0 * cbor_packed_total as f64 / json_total as f64
    );
    println!(
        "  CBOR IntKey:    {:>6} bytes ({:>5.1}%)",
        cbor_intkey_total / n,
        100.0 * cbor_intkey_total as f64 / json_total as f64
    );
    println!(
        "  Proto String:   {:>6} bytes ({:>5.1}%)",
        proto_string_total / n,
        100.0 * proto_string_total as f64 / json_total as f64
    );
    println!(
        "  Proto Binary:   {:>6} bytes ({:>5.1}%)",
        proto_binary_total / n,
        100.0 * proto_binary_total as f64 / json_total as f64
    );
    println!(
        "  Cap'n Proto:    {:>6} bytes ({:>5.1}%)",
        capnp_total / n,
        100.0 * capnp_total as f64 / json_total as f64
    );
    println!(
        "  Notepack:       {:>6} bytes ({:>5.1}%)",
        notepack_total / n,
        100.0 * notepack_total as f64 / json_total as f64
    );
}

fn bench_kind_0_profile(c: &mut Criterion) {
    bench_kind(c, 0, "profile");
}

fn bench_kind_1_notes(c: &mut Criterion) {
    bench_kind(c, 1, "notes");
}

fn bench_kind_3_follows(c: &mut Criterion) {
    bench_kind(c, 3, "follows");
}

fn bench_kind_4_dms(c: &mut Criterion) {
    bench_kind(c, 4, "dms");
}

fn bench_kind_7_reactions(c: &mut Criterion) {
    bench_kind(c, 7, "reactions");
}

fn bench_kind_10002_relays(c: &mut Criterion) {
    bench_kind(c, 10002, "relays");
}

fn bench_kind_30023_articles(c: &mut Criterion) {
    bench_kind(c, 30023, "articles");
}

criterion_group! {
    name = benches;
    config = common::auto_criterion();
    targets = bench_kind_0_profile, bench_kind_1_notes, bench_kind_3_follows,
              bench_kind_4_dms, bench_kind_7_reactions, bench_kind_10002_relays,
              bench_kind_30023_articles
}
criterion_main!(benches);
