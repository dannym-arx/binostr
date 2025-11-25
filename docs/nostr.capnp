@0xb7c8f8a9d3e2f1a0;

# Cap'n Proto schema for Nostr events
# Binary-optimized: id, pubkey, sig stored as raw bytes

struct NostrEvent {
  id @0 :Data;          # 32 bytes
  pubkey @1 :Data;      # 32 bytes
  createdAt @2 :Int64;
  kind @3 :UInt32;
  tags @4 :List(Tag);
  content @5 :Text;
  sig @6 :Data;         # 64 bytes
}

struct Tag {
  values @0 :List(Text);
}

struct EventBatch {
  events @0 :List(NostrEvent);
}

