# Binostr

A benchmarking library for comparing binary serialization formats for [Nostr](https://github.com/nostr-protocol/nips) events.

This project evaluates **JSON**, **CBOR**, **Protocol Buffers**, **Cap'n Proto**, **DannyPack**, and **Notepack** to inform potential NIPs for binary client-relay communication.

## Key Findings

### TL;DR

| Priority | Winner | Details |
|----------|--------|---------|
| **Speed (Serialize)** | Cap'n Proto | 3.6x faster than JSON |
| **Speed (Deserialize)** | Proto Binary | 1.4x faster than JSON |
| **Size** | CBOR Packed | 12% smaller than JSON (raw) |
| **Overall** | **Proto Binary** | Best balance of speed and size |

### Size Comparison

| Format | Raw Size | vs JSON | After Zstd | vs JSON (Zstd) |
|--------|----------|---------|------------|----------------|
| **CBOR Packed** | 2,185 bytes | **88.0%** | 1,271 bytes | 97.9% |
| CBOR IntKey | 2,192 bytes | 88.3% | 1,280 bytes | 98.6% |
| **Proto Binary** | 2,227 bytes | **89.7%** | 1,282 bytes | 98.8% |
| CBOR Schemaless | 2,228 bytes | 89.7% | 1,310 bytes | 100.9% |
| Proto String | 2,356 bytes | 94.9% | 1,264 bytes | 97.4% |
| JSON (baseline) | 2,482 bytes | 100% | 1,298 bytes | 100% |
| Cap'n Proto | 3,035 bytes | 122.3% | 1,428 bytes | 110.0% |

> **Important**: After compression (gzip/zstd), all formats are within ~10% of each other. Cap'n Proto trades size for speed. The main benefit of binary formats becomes **parsing speed**.

### Speed Comparison

| Format | Serialize | Deserialize | Notes |
|--------|-----------|-------------|-------|
| **Cap'n Proto** | **254 ns** | 2,043 ns | Fastest serialize, no encoding step |
| **Proto Binary** | 350 ns | **1,943 ns** | Fastest deserialize |
| CBOR Schemaless | 573 ns | 3,089 ns | |
| CBOR Packed | 610 ns | 4,742 ns | |
| CBOR IntKey | 658 ns | 4,890 ns | |
| Proto String | 709 ns | 2,452 ns | |
| JSON (baseline) | 923 ns | 2,713 ns | |

### Per-Kind Analysis

Different event types show different savings:

| Kind | Name | Best Format | Savings vs JSON |
|------|------|-------------|-----------------|
| 0 | Profile Metadata | Proto Binary | **37.6%** |
| 7 | Reaction | CBOR Packed | **38.7%** |
| 1 | Short Text Note | CBOR Packed | **19.3%** |
| 3 | Follow List | CBOR Packed | **7.2%** |
| 30023 | Long-form Article | CBOR Packed | **3.3%** |

Events with more fixed-size fields (id, pubkey, sig) benefit more from binary encoding. Content-heavy events (articles) show minimal savings since text compresses similarly regardless of format.

## Formats Tested

### JSON (Baseline)
Standard NIP-01 JSON format with hex-encoded cryptographic fields.

### Protocol Buffers
- **Proto String**: Hex-encoded id/pubkey/sig (compatible)
- **Proto Binary**: Raw bytes for id/pubkey/sig (saves 128 bytes/event)

### CBOR
- **Schemaless**: JSON-like with string field names
- **Packed Array**: Positional encoding `[id, pubkey, created_at, kind, tags, content, sig]`
- **Integer-Keyed Map**: `{0: id, 1: pubkey, ...}` for extensibility

All CBOR variants use hex-to-binary optimization for tag values (e.g., event IDs in `e` tags are stored as 32 bytes instead of 64 hex characters).

### Cap'n Proto
- Zero-copy serialization format - the wire format IS the in-memory representation
- Extremely fast serialization (~254ns) because there's no encoding step
- Larger size due to alignment/padding for direct memory access
- Supports selective field access without full deserialization
- See [capnproto.org](https://capnproto.org/) for details

### DannyPack
Custom binary format designed specifically for Nostr events:
- Fixed 138-byte header for cryptographic fields and metadata
- Varint encoding for compact length prefixes
- Automatic hex-to-binary conversion for tag values
- Ultra-fast serialization using unsafe pointer operations
- Safe variant (`deserialize_safe`) available for untrusted input

See `src/dannypack.rs` for detailed wire format documentation.

### Notepack
Compact binary format designed specifically for Nostr notes:
- Varint encoding for integers (LEB128-style)
- Hex strings stored as raw bytes (32-byte pubkeys stored as 32 bytes, not 64 hex chars)
- Streaming parser for memory-efficient processing
- Base64-prefixed string format (`notepack_...`) for easy transport
- Zero-allocation parsing via lazy tag iterators

See [notepack on crates.io](https://crates.io/crates/notepack) for details.

## Usage

### Run Size Analysis

```bash
# Analyze event distribution and sizes
cargo run --example analyze_data

# Size report for specific event kind
cargo run --example size_report -- --kind 3
```

### Run Benchmarks

```bash
# Serialization speed
cargo bench --bench serialize

# Deserialization speed
cargo bench --bench deserialize

# Per-kind analysis (profile, notes, follows, etc.)
cargo bench --bench by_kind

# Per-category analysis (size and tag count categories)
cargo bench --bench by_category

# Zero-copy field access (Cap'n Proto's advantage)
cargo bench --bench zero_copy

# Size comparison report
cargo bench --bench size_analysis

# For faster iteration during development (less statistically rigorous):
BINOSTR_FAST_BENCH=1 cargo bench
```

### Quick Benchmark Report

```bash
# Run comprehensive benchmark with comparison tables
cargo run --release --example bench_report
```

This produces a single report comparing all formats on serialization speed, deserialization speed, and wire size with rankings and recommendations.

### Analysis Tools

```bash
# Batch overhead analysis
cargo run --example batch_analysis
```

## Benchmark Methodology

### Test Environment

These benchmarks were run on:

- **CPU**: Apple M4 Max (14 cores)
- **RAM**: 36 GB
- **OS**: macOS 15.1 (Darwin 25.1.0)
- **Rust**: 1.90.0

### Best Practices for Reproducible Results

For accurate benchmark results:

1. **Close other applications** - Background processes can cause variance
2. **Disable Turbo Boost** (if possible) - Prevents thermal throttling
3. **Run multiple times** - Criterion automatically runs 100 samples
4. **Use release mode** - `cargo bench` automatically uses `--release`
5. **Let the system stabilize** - Wait a minute after boot before benchmarking
6. **Plug in power** (laptops) - Battery mode may throttle CPU

### Data Source

Benchmarks use real Nostr events from `.pb.gz` files in the `data/` directory:
- ~50,000 events sampled across 8 days
- Natural distribution of event kinds (kind 1 notes, reactions, follow lists, etc.)
- Representative mix of sizes (tiny reactions to large follow lists)

### Statistical Rigor

[Criterion.rs](https://github.com/bheisler/criterion.rs) provides:
- Warm-up periods to stabilize CPU caches
- 100 samples per benchmark
- Outlier detection and reporting
- Statistical significance analysis
- HTML reports in `target/criterion/`

## Recommendation for NIP

Based on these benchmarks, **Protocol Buffers (Binary)** is recommended for a binary Nostr NIP:

### Pros
- **Fast parsing** - 1.4x faster than JSON for deserialization
- **~10% smaller** than JSON (raw)
- Excellent cross-language tooling (official support for 10+ languages)
- Schema provides documentation and type safety
- Well-established in production systems (Google, gRPC, etc.)

### Cons
- Requires schema compilation step
- Binary format harder to debug visually
- Slightly larger than CBOR Packed (~1.7% difference)

### Alternative: Cap'n Proto

If **maximum serialization speed** is critical (high-throughput relays writing to disk):
- 3.6x faster serialization than JSON
- Zero-copy reads possible
- But ~22% larger raw size
- Compresses well but still ~10% larger after compression

### Alternative: CBOR Packed

If schema-less encoding is preferred:
- Smallest raw size (12% smaller than JSON)
- No compilation step needed
- Self-describing format
- But 2x slower than Proto Binary

## Project Structure

```
binostr/
├── src/
│   ├── lib.rs          # Library exports
│   ├── event.rs        # NostrEvent struct
│   ├── loader.rs       # .pb.gz file loader
│   ├── sampler.rs      # Random sampling with excluded kinds
│   ├── json.rs         # JSON serialization
│   ├── cbor.rs         # CBOR variants (with hex optimization)
│   ├── proto.rs        # Protobuf variants
│   ├── capnp.rs        # Cap'n Proto (with zero-copy field access)
│   ├── dannypack.rs    # Custom binary format (safe & unsafe variants)
│   ├── notepack.rs     # Notepack format (compact with streaming parser)
│   └── stats.rs        # Analysis utilities & compression helpers
├── benches/
│   ├── serialize.rs    # Serialization speed benchmarks
│   ├── deserialize.rs  # Deserialization speed benchmarks
│   ├── by_kind.rs      # Per-kind benchmarks (profile, notes, etc.)
│   ├── by_category.rs  # Per-category benchmarks (size, tag count)
│   ├── zero_copy.rs    # Zero-copy field access benchmarks
│   ├── size_analysis.rs # Size comparison report
│   └── common.rs       # Shared benchmark utilities
├── tests/
│   └── roundtrip.rs    # Comprehensive roundtrip tests
├── examples/
│   ├── analyze_data.rs # Event distribution analysis
│   ├── size_report.rs  # Size comparison report
│   └── batch_analysis.rs # Batch overhead analysis
└── docs/
    ├── nostr.proto         # Original protobuf schema
    ├── nostr_binary.proto  # Binary-optimized schema
    ├── nostr.cddl          # CBOR schema (CDDL)
    └── nostr.capnp         # Cap'n Proto schema
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! Please run `cargo clippy` and `cargo fmt` before submitting PRs.
