//! Generate a detailed serialization benchmark report
//!
//! Run with: cargo run --release --example bench_report
//!
//! Optional arguments:
//!   cargo run --release --example bench_report -- --sample-size 10000
//!   cargo run --release --example bench_report -- --iterations 100

use std::env;
use std::time::{Duration, Instant};

use binostr::sampler::EventSampler;
use binostr::stats::Format;
use binostr::{capnp, cbor, dannypack, json, proto, NostrEvent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let sample_size = parse_arg(&args, "--sample-size").unwrap_or(1_000);
    let iterations = parse_arg(&args, "--iterations").unwrap_or(100);

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              BINOSTR BENCHMARK REPORT                            ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    println!("Loading {} events from data directory...", sample_size);
    let mut sampler = EventSampler::from_directory("data", sample_size * 2)?;
    let events: Vec<NostrEvent> = sampler
        .random_sample(sample_size)
        .into_iter()
        .cloned()
        .collect();

    println!(
        "Benchmarking {} events × {} iterations...",
        events.len(),
        iterations
    );
    println!();

    // Warmup
    for event in events.iter().take(10) {
        let _ = json::serialize(event);
        let _ = dannypack::serialize(event);
    }

    // Benchmark serialization
    let ser_results = benchmark_serialize(&events, iterations);

    // Pre-serialize for deserialization benchmarks
    let json_data: Vec<_> = events.iter().map(json::serialize).collect();
    let cbor_schemaless_data: Vec<_> = events.iter().map(cbor::schemaless::serialize).collect();
    let cbor_packed_data: Vec<_> = events.iter().map(cbor::packed::serialize).collect();
    let cbor_intkey_data: Vec<_> = events.iter().map(cbor::intkey::serialize).collect();
    let proto_string_data: Vec<_> = events.iter().map(proto::string::serialize).collect();
    let proto_binary_data: Vec<_> = events.iter().map(proto::binary::serialize).collect();
    let capnp_data: Vec<_> = events.iter().map(capnp::serialize_event).collect();
    let capnp_packed_data: Vec<_> = events.iter().map(capnp::serialize_event_packed).collect();
    let dannypack_data: Vec<_> = events.iter().map(dannypack::serialize).collect();

    // Benchmark deserialization
    let deser_results = benchmark_deserialize(
        &json_data,
        &cbor_schemaless_data,
        &cbor_packed_data,
        &cbor_intkey_data,
        &proto_string_data,
        &proto_binary_data,
        &capnp_data,
        &capnp_packed_data,
        &dannypack_data,
        iterations,
    );

    // Print results
    print_serialize_results(&ser_results, events.len());
    println!();
    print_deserialize_results(&deser_results, events.len());
    println!();

    // Summary
    print_summary(&ser_results, &deser_results, events.len());

    Ok(())
}

struct BenchResult {
    format: Format,
    total_time: Duration,
    min_time: Duration,
    max_time: Duration,
}

fn benchmark_serialize(events: &[NostrEvent], iterations: usize) -> Vec<BenchResult> {
    let formats: Vec<(Format, Box<dyn Fn(&NostrEvent) -> Vec<u8>>)> = vec![
        (Format::Json, Box::new(|e| json::serialize(e))),
        (
            Format::CborSchemaless,
            Box::new(|e| cbor::schemaless::serialize(e)),
        ),
        (Format::CborPacked, Box::new(|e| cbor::packed::serialize(e))),
        (Format::CborIntKey, Box::new(|e| cbor::intkey::serialize(e))),
        (
            Format::ProtoString,
            Box::new(|e| proto::string::serialize(e)),
        ),
        (
            Format::ProtoBinary,
            Box::new(|e| proto::binary::serialize(e)),
        ),
        (Format::CapnProto, Box::new(|e| capnp::serialize_event(e))),
        (
            Format::CapnProtoPacked,
            Box::new(|e| capnp::serialize_event_packed(e)),
        ),
        (Format::DannyPack, Box::new(|e| dannypack::serialize(e))),
    ];

    let mut results = Vec::new();

    for (format, serialize_fn) in &formats {
        let mut total = Duration::ZERO;
        let mut min = Duration::MAX;
        let mut max = Duration::ZERO;

        for _ in 0..iterations {
            let start = Instant::now();
            for event in events {
                std::hint::black_box(serialize_fn(event));
            }
            let elapsed = start.elapsed();
            total += elapsed;
            min = min.min(elapsed);
            max = max.max(elapsed);
        }

        results.push(BenchResult {
            format: *format,
            total_time: total,
            min_time: min,
            max_time: max,
        });
    }

    // Sort by total time (fastest first)
    results.sort_by_key(|r| r.total_time);
    results
}

fn benchmark_deserialize(
    json_data: &[Vec<u8>],
    cbor_schemaless_data: &[Vec<u8>],
    cbor_packed_data: &[Vec<u8>],
    cbor_intkey_data: &[Vec<u8>],
    proto_string_data: &[Vec<u8>],
    proto_binary_data: &[Vec<u8>],
    capnp_data: &[Vec<u8>],
    capnp_packed_data: &[Vec<u8>],
    dannypack_data: &[Vec<u8>],
    iterations: usize,
) -> Vec<BenchResult> {
    let mut results = Vec::new();

    // JSON
    results.push(bench_deser(Format::Json, iterations, || {
        for data in json_data {
            std::hint::black_box(json::deserialize(data).unwrap());
        }
    }));

    // CBOR Schemaless
    results.push(bench_deser(Format::CborSchemaless, iterations, || {
        for data in cbor_schemaless_data {
            std::hint::black_box(cbor::schemaless::deserialize(data).unwrap());
        }
    }));

    // CBOR Packed
    results.push(bench_deser(Format::CborPacked, iterations, || {
        for data in cbor_packed_data {
            std::hint::black_box(cbor::packed::deserialize(data).unwrap());
        }
    }));

    // CBOR IntKey
    results.push(bench_deser(Format::CborIntKey, iterations, || {
        for data in cbor_intkey_data {
            std::hint::black_box(cbor::intkey::deserialize(data).unwrap());
        }
    }));

    // Proto String
    results.push(bench_deser(Format::ProtoString, iterations, || {
        for data in proto_string_data {
            std::hint::black_box(proto::string::deserialize(data).unwrap());
        }
    }));

    // Proto Binary
    results.push(bench_deser(Format::ProtoBinary, iterations, || {
        for data in proto_binary_data {
            std::hint::black_box(proto::binary::deserialize(data).unwrap());
        }
    }));

    // Cap'n Proto
    results.push(bench_deser(Format::CapnProto, iterations, || {
        for data in capnp_data {
            std::hint::black_box(capnp::deserialize_event(data).unwrap());
        }
    }));

    // Cap'n Proto Packed
    results.push(bench_deser(Format::CapnProtoPacked, iterations, || {
        for data in capnp_packed_data {
            std::hint::black_box(capnp::deserialize_event_packed(data).unwrap());
        }
    }));

    // DannyPack
    results.push(bench_deser(Format::DannyPack, iterations, || {
        for data in dannypack_data {
            std::hint::black_box(dannypack::deserialize(data).unwrap());
        }
    }));

    // Sort by total time (fastest first)
    results.sort_by_key(|r| r.total_time);
    results
}

fn bench_deser<F: FnMut()>(format: Format, iterations: usize, mut f: F) -> BenchResult {
    let mut total = Duration::ZERO;
    let mut min = Duration::MAX;
    let mut max = Duration::ZERO;

    for _ in 0..iterations {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed();
        total += elapsed;
        min = min.min(elapsed);
        max = max.max(elapsed);
    }

    BenchResult {
        format,
        total_time: total,
        min_time: min,
        max_time: max,
    }
}

fn print_serialize_results(results: &[BenchResult], event_count: usize) {
    let fastest = results.first().map(|r| r.total_time).unwrap_or_default();

    println!("📤 SERIALIZATION");
    println!("┌──────────────────┬────────────┬────────────┬────────────┬─────────────┬─────────┐");
    println!("│ Format           │ Total Time │  Min/iter  │  Max/iter  │  Per Event  │ vs Best │");
    println!("├──────────────────┼────────────┼────────────┼────────────┼─────────────┼─────────┤");

    for result in results {
        let per_event = result.total_time.as_nanos() as f64
            / (event_count as f64 * results.len() as f64 / results.len() as f64)
            / 100.0; // divide by iterations
        let vs_best = 100.0 * result.total_time.as_nanos() as f64 / fastest.as_nanos() as f64;

        println!(
            "│ {:16} │ {:>10} │ {:>10} │ {:>10} │ {:>9.0} ns │ {:>6.0}% │",
            result.format.name(),
            format_duration(result.total_time),
            format_duration(result.min_time),
            format_duration(result.max_time),
            per_event,
            vs_best
        );
    }

    println!("└──────────────────┴────────────┴────────────┴────────────┴─────────────┴─────────┘");
}

fn print_deserialize_results(results: &[BenchResult], event_count: usize) {
    let fastest = results.first().map(|r| r.total_time).unwrap_or_default();

    println!("📥 DESERIALIZATION");
    println!("┌──────────────────┬────────────┬────────────┬────────────┬─────────────┬─────────┐");
    println!("│ Format           │ Total Time │  Min/iter  │  Max/iter  │  Per Event  │ vs Best │");
    println!("├──────────────────┼────────────┼────────────┼────────────┼─────────────┼─────────┤");

    for result in results {
        let per_event = result.total_time.as_nanos() as f64
            / (event_count as f64 * results.len() as f64 / results.len() as f64)
            / 100.0;
        let vs_best = 100.0 * result.total_time.as_nanos() as f64 / fastest.as_nanos() as f64;

        println!(
            "│ {:16} │ {:>10} │ {:>10} │ {:>10} │ {:>9.0} ns │ {:>6.0}% │",
            result.format.name(),
            format_duration(result.total_time),
            format_duration(result.min_time),
            format_duration(result.max_time),
            per_event,
            vs_best
        );
    }

    println!("└──────────────────┴────────────┴────────────┴────────────┴─────────────┴─────────┘");
}

fn print_summary(ser_results: &[BenchResult], deser_results: &[BenchResult], event_count: usize) {
    let ser_best = ser_results.first().unwrap();
    let deser_best = deser_results.first().unwrap();

    // Find JSON for comparison
    let json_ser = ser_results
        .iter()
        .find(|r| r.format == Format::Json)
        .unwrap();
    let json_deser = deser_results
        .iter()
        .find(|r| r.format == Format::Json)
        .unwrap();

    let ser_speedup = json_ser.total_time.as_nanos() as f64 / ser_best.total_time.as_nanos() as f64;
    let deser_speedup =
        json_deser.total_time.as_nanos() as f64 / deser_best.total_time.as_nanos() as f64;

    println!("📊 SUMMARY");
    println!("┌────────────────────────────────────────────────────────────────────┐");
    println!(
        "│ Fastest Serialize:   {:16} ({:.1}x faster than JSON)     │",
        ser_best.format.name(),
        ser_speedup
    );
    println!(
        "│ Fastest Deserialize: {:16} ({:.1}x faster than JSON)     │",
        deser_best.format.name(),
        deser_speedup
    );
    println!("├────────────────────────────────────────────────────────────────────┤");

    // Combined throughput (serialize + deserialize)
    println!(
        "│ Combined Throughput (events/sec @ {} events):                   │",
        event_count
    );

    let mut combined: Vec<_> = Format::all()
        .iter()
        .filter_map(|&f| {
            let ser = ser_results.iter().find(|r| r.format == f)?;
            let deser = deser_results.iter().find(|r| r.format == f)?;
            let total_ns = ser.total_time.as_nanos() + deser.total_time.as_nanos();
            let events_per_sec = (event_count as f64 * 100.0) / (total_ns as f64 / 1_000_000_000.0);
            Some((f, events_per_sec))
        })
        .collect();

    combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    for (format, throughput) in &combined {
        println!(
            "│   {:16}: {:>12.0} events/sec                         │",
            format.name(),
            throughput
        );
    }

    println!("└────────────────────────────────────────────────────────────────────┘");
}

fn parse_arg<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
}

fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos >= 1_000_000_000 {
        format!("{:.2} s", d.as_secs_f64())
    } else if nanos >= 1_000_000 {
        format!("{:.2} ms", nanos as f64 / 1_000_000.0)
    } else if nanos >= 1_000 {
        format!("{:.2} µs", nanos as f64 / 1_000.0)
    } else {
        format!("{} ns", nanos)
    }
}
