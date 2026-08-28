fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        // build_server(true): 生成 server trait，供集成测试 mock Mind 服务使用
        // （Anaphase 本身仅作 client，不启用 server；trait 仅供测试实现）
        .build_server(true)
        .compile(
            &[
                "proto/helix_mind.proto",
                "proto/flowmodus.proto",
                "proto/tentacle.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
