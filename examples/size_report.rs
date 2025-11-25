//! Generate a detailed size comparison report
//!
//! Run with: cargo run --example size_report
//!
//! Optional arguments:
//!   cargo run --example size_report -- --sample-size 10000
//!   cargo run --example size_report -- --kind 3

use std::env;

use binostr::sampler::EventSampler;
use binostr::stats::{compute_size_stats, Format};
use binostr::NostrEvent;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let sample_size = parse_arg(&args, "--sample-size").unwrap_or(10_000);
    let filter_kind: Option<u16> = parse_arg(&args, "--kind");

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║               BINOSTR SIZE COMPARISON REPORT                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    println!("Loading {} events from data directory...", sample_size);
    let mut sampler = EventSampler::from_directory("data", sample_size * 2)?;

    let events: Vec<NostrEvent> = if let Some(kind) = filter_kind {
        println!("Filtering to kind {}...", kind);
        sampler
            .sample_kind(kind, sample_size)
            .into_iter()
            .cloned()
            .collect()
    } else {
        sampler
            .random_sample(sample_size)
            .into_iter()
            .cloned()
            .collect()
    };

    println!("Analyzing {} events...", events.len());
    println!();

    // Aggregate statistics
    let mut totals: Vec<(Format, usize, usize, usize)> =
        Format::all().iter().map(|&f| (f, 0, 0, 0)).collect();

    for event in &events {
        let stats = compute_size_stats(event);
        for stat in stats {
            if let Some(entry) = totals.iter_mut().find(|(f, _, _, _)| *f == stat.format) {
                entry.1 += stat.raw_bytes;
                entry.2 += stat.gzip_bytes;
                entry.3 += stat.zstd_bytes;
            }
        }
    }

    // Sort by raw size
    totals.sort_by_key(|(_, raw, _, _)| *raw);

    let json_total = totals
        .iter()
        .find(|(f, _, _, _)| *f == Format::Json)
        .map(|(_, raw, _, _)| *raw)
        .unwrap_or(1);

    println!("┌──────────────────┬────────────┬────────────┬────────────┬─────────┐");
    println!("│ Format           │ Total Raw  │ Total Gzip │ Total Zstd │ vs JSON │");
    println!("├──────────────────┼────────────┼────────────┼────────────┼─────────┤");

    for (format, raw, gzip, zstd) in &totals {
        let vs_json = 100.0 * *raw as f64 / json_total as f64;
        println!(
            "│ {:16} │ {:>10} │ {:>10} │ {:>10} │ {:>6.1}% │",
            format.name(),
            format_bytes(*raw),
            format_bytes(*gzip),
            format_bytes(*zstd),
            vs_json
        );
    }

    println!("└──────────────────┴────────────┴────────────┴────────────┴─────────┘");
    println!();

    // Per-event average
    let n = events.len();
    println!("Average per event:");
    println!("┌──────────────────┬──────────┬──────────┬──────────┐");
    println!("│ Format           │ Avg Raw  │ Avg Gzip │ Avg Zstd │");
    println!("├──────────────────┼──────────┼──────────┼──────────┤");

    for (format, raw, gzip, zstd) in &totals {
        println!(
            "│ {:16} │ {:>8} │ {:>8} │ {:>8} │",
            format.name(),
            raw / n,
            gzip / n,
            zstd / n
        );
    }

    println!("└──────────────────┴──────────┴──────────┴──────────┘");
    println!();

    // Savings summary
    let json_raw = totals
        .iter()
        .find(|(f, _, _, _)| *f == Format::Json)
        .unwrap()
        .1;
    let best = totals.first().unwrap();

    let savings = json_raw - best.1;
    let savings_pct = 100.0 * savings as f64 / json_raw as f64;

    println!("📊 Summary:");
    println!(
        "   Best format: {} ({:.1}% smaller than JSON)",
        best.0.name(),
        savings_pct
    );
    println!(
        "   Total savings: {} per {} events",
        format_bytes(savings),
        n
    );
    println!("   Per-event savings: {} bytes", savings / n);
    println!();

    // Compression effectiveness
    println!("📦 Compression Effectiveness:");
    for (format, raw, gzip, zstd) in &totals {
        let gzip_ratio = 100.0 * *gzip as f64 / *raw as f64;
        let zstd_ratio = 100.0 * *zstd as f64 / *raw as f64;
        println!(
            "   {:16}: gzip={:>5.1}%, zstd={:>5.1}%",
            format.name(),
            gzip_ratio,
            zstd_ratio
        );
    }

    Ok(())
}

fn parse_arg<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
}

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.2} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.2} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}
