//! Analyze the event data files
//!
//! Run with: cargo run --example analyze_data

use binostr::sampler::EventSampler;
use binostr::stats::{generate_size_report, DistributionAnalysis};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading events from data directory...");

    // Load a sample of events
    let mut sampler = EventSampler::from_directory("data", 50_000)?;

    println!("Loaded {} events", sampler.len());
    println!();

    // Distribution analysis
    let dist = DistributionAnalysis::from_events(sampler.all());

    println!("=== Distribution Analysis ===");
    println!("Total events: {}", dist.total_events);
    println!("Average content length: {:.1} bytes", dist.avg_content_len);
    println!("Average tag count: {:.1}", dist.avg_tag_count);
    println!();

    println!("=== Top 20 Event Kinds ===");
    for (kind, count) in dist.top_kinds(20) {
        let pct = 100.0 * count as f64 / dist.total_events as f64;
        let name = kind_name(kind);
        println!(
            "  Kind {:>5}: {:>6} events ({:>5.1}%) - {}",
            kind, count, pct, name
        );
    }
    println!();

    println!("=== Size Distribution ===");
    for (cat, count) in &dist.by_size {
        let pct = 100.0 * *count as f64 / dist.total_events as f64;
        println!(
            "  {:>20}: {:>6} events ({:>5.1}%)",
            format!("{}", cat),
            count,
            pct
        );
    }
    println!();

    println!("=== Tag Count Distribution ===");
    for (cat, count) in &dist.by_tags {
        let pct = 100.0 * *count as f64 / dist.total_events as f64;
        println!(
            "  {:>20}: {:>6} events ({:>5.1}%)",
            format!("{}", cat),
            count,
            pct
        );
    }
    println!();

    // Generate size report for a smaller sample
    let sample: Vec<_> = sampler.random_sample(10_000).into_iter().cloned().collect();
    println!("=== Size Comparison Report (10000 random events) ===");
    println!("{}", generate_size_report(&sample));

    Ok(())
}

fn kind_name(kind: u32) -> &'static str {
    match kind {
        0 => "Profile Metadata",
        1 => "Short Text Note",
        2 => "Recommend Relay",
        3 => "Follow List",
        4 => "Encrypted DM",
        5 => "Event Deletion",
        6 => "Repost",
        7 => "Reaction",
        8 => "Badge Award",
        9 => "Group Chat Message",
        10 => "Group Chat Threaded Reply",
        11 => "Group Thread",
        12 => "Group Thread Reply",
        13 => "Seal",
        14 => "Direct Message",
        16 => "Generic Repost",
        17 => "Reaction to Website",
        40 => "Channel Creation",
        41 => "Channel Metadata",
        42 => "Channel Message",
        43 => "Channel Hide Message",
        44 => "Channel Mute User",
        1059 => "Gift Wrap",
        1063 => "File Metadata",
        1311 => "Live Chat Message",
        1984 => "Reporting",
        1985 => "Label",
        4550 => "Community Post Approval",
        5000..=5999 => "Job Request",
        6000..=6999 => "Job Result",
        7000 => "Job Feedback",
        9041 => "Zap Goal",
        9734 => "Zap Request",
        9735 => "Zap",
        10000 => "Mute List",
        10001 => "Pin List",
        10002 => "Relay List Metadata",
        10003 => "Bookmark List",
        10004 => "Communities List",
        10005 => "Public Chats List",
        10006 => "Blocked Relays List",
        10007 => "Search Relays List",
        10015 => "Interests List",
        10030 => "User Emoji List",
        10096 => "File Storage Server List",
        13194 => "Wallet Info",
        21000 => "Lightning Pub RPC",
        22242 => "Client Authentication",
        23194 => "Wallet Request",
        23195 => "Wallet Response",
        24133 => "Nostr Connect",
        27235 => "HTTP Auth",
        30000 => "Follow Sets",
        30001 => "Generic Lists",
        30002 => "Relay Sets",
        30003 => "Bookmark Sets",
        30004 => "Curation Sets",
        30008 => "Profile Badges",
        30009 => "Badge Definition",
        30015 => "Interest Sets",
        30017 => "Create/Update Stall",
        30018 => "Create/Update Product",
        30023 => "Long-form Content",
        30024 => "Draft Long-form",
        30030 => "Emoji Sets",
        30078 => "Application-specific Data",
        30311 => "Live Event",
        30315 => "User Statuses",
        30402 => "Classified Listing",
        30403 => "Draft Classified",
        31922 => "Date-Based Calendar Event",
        31923 => "Time-Based Calendar Event",
        31924 => "Calendar",
        31925 => "Calendar Event RSVP",
        31989 => "Handler Recommendation",
        31990 => "Handler Information",
        34550 => "Community Definition",
        _ => "Unknown",
    }
}
