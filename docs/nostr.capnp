@0xb7c8f8a9d3e2f1a0;

# Cap'n Proto schema for Nostr events - MAXIMUM COMPRESSION
#
# All fixed-size fields packed into single blob (138 bytes):
#   - id: 32 bytes (offset 0)
#   - pubkey: 32 bytes (offset 32)  
#   - sig: 64 bytes (offset 64)
#   - createdAt: 8 bytes i64 LE (offset 128)
#   - kind: 2 bytes u16 LE (offset 136)
#
# This eliminates ALL Cap'n Proto struct/field overhead for fixed data.
# Only 3 pointers remain: fixedData, tagData, content

struct NostrEvent {
  # All fixed fields packed into 138 bytes
  fixedData @0 :Data;
  
  # Packed tags: [tag_count:u16] then per tag: [value_count:u8] then per value:
  #              [flags_and_len:u16 where bit15=is_hex, bits0-14=length][data]
  tagData @1 :Data;
  
  # Variable-length content
  content @2 :Text;
}

struct EventBatch {
  events @0 :List(NostrEvent);
}

