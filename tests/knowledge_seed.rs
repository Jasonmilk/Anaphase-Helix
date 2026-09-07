//! L2 knowledge seeding (manual, needs a live Helix-Mind at :50052).
//!
//! The three founding laws are written as L2 knowledge nodes in Mind's own
//! language: claim + applicability boundary (where it holds, where it does
//! not). Boundaries are part of the knowledge — Helix cites edges, never
//! invents them (ADR-0018 rail philosophy applied to facts).
use anaphase::adapters::mind::GrpcMindAdapter;
use anaphase::adapters::MemoryAdapter;

const MIND: &str = "http://127.0.0.1:50052";

#[tokio::test]
#[ignore]
async fn seed_l2_three_laws() {
    let adapter = GrpcMindAdapter::new(MIND, Default::default())
        .await
        .unwrap();
    let laws = [
        "熵增定律（热力学第二定律）：孤立系统的熵永不减少，能量自发从有序走向无序，一切真实过程不可逆。适用：宏观热力学孤立系统。不适用：开放系统在持续能量输入下的局部有序（如生命）。",
        "质量守恒定律：封闭系统内，化学反应前后质量总和不变，物质不灭，仅转化形式。适用：化学变化。不适用：核反应（质能等价，E=mc²）。",
        "万有引力定律：任意两质点相互吸引，引力与质量乘积成正比、与距离平方成反比。适用：经典弱引力场。不适用：强引力场（需广义相对论修正）。",
    ];
    for law in laws {
        adapter.remember_node(law, 2).await.unwrap();
    }
}
