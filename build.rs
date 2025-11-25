use std::io::Result;

fn main() -> Result<()> {
    // Compile Protocol Buffers schemas
    prost_build::compile_protos(&["docs/nostr.proto", "docs/nostr_binary.proto"], &["docs/"])?;

    // Compile Cap'n Proto schema
    capnpc::CompilerCommand::new()
        .src_prefix("docs")
        .file("docs/nostr.capnp")
        .default_parent_module(vec!["capnp".into()])
        .run()
        .expect("capnp schema compilation failed");

    Ok(())
}
