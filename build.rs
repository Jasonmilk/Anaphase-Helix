fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile(
            &[
                "proto/helix_mind.proto",
                "proto/flowmodus.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
