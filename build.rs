fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../helix-mind/crates/helix-mind-api/proto/helix_mind.proto")?;
    Ok(())
}
