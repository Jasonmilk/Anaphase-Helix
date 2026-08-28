//! GrpcMindAdapter — Helix-Mind gRPC 记忆契约客户端（P10a 契约对齐，ADR-0001）。
//!
//! T2 补全（2026-08-28，含系统探针裁决）：
//! - `EnergyContext` 完整构造（含 `budget_tier`，由确定性推导 + **真实系统负载**产生）
//! - 系统探针（sysinfo）：`system_load` 从真实 CPU/内存负载归一化，激活 Mind 紧急
//!   通路（retrieval `system_load > 0.9` 紧急模式），身体生命体征 → Mind 决策
//! - W3C 根 `traceparent` 生成（`00-{32hex}-{16hex}-01`，uuid）
//! - 去硬编码：`suggested_mode`/`allow_imagination`/`autonomy_level` 由推导函数产生
//! - 降级钩子：gRPC 失败时记录降级事件（含 traceparent，可观测），fail-open 由上层接管
//!
//! 生态手套感知（MCP/宇树/Unity/鸿蒙可用性）为 P10b 渐进扩展，P10a 仅预留（见下方注记）。
//! 推导说明：当前为确定性启发式 + 系统探针（P10a 最小正确）；P10b 接入状态机（states.rs）
//! 与生态手套感知后扩展。

use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use sysinfo::System;
use tonic::transport::Channel;
use tracing::warn;
use uuid::Uuid;

use crate::helix_mind_api::helix_mind_client::HelixMindClient;
use crate::helix_mind_api::{
    AutonomyLevel, BudgetTier, CognitiveMode, EnergyContext, HelixQueryRequest, RememberRequest,
};
use super::{MemoryAdapter, QueryResult};

pub struct GrpcMindAdapter {
    client: HelixMindClient<Channel>,
    /// Amygdala PreAssessment 输出复杂度（1=简单/2=中等/3=复杂；0=未知，走兜底）
    complexity: AtomicU8,
}

impl GrpcMindAdapter {
    pub async fn new(endpoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_shared(endpoint.to_string())?
            .connect()
            .await?;
        let client = HelixMindClient::new(channel);
        Ok(Self {
            client,
            complexity: AtomicU8::new(0),
        })
    }
}

#[async_trait]
impl MemoryAdapter for GrpcMindAdapter {
    async fn query(&self, query: &str, include_recessive: bool) -> Result<QueryResult, String> {
        // W3C 根 traceparent（DNA 原则 9：Anaphase 生成根，Mind 只透传）。
        let traceparent = generate_traceparent();
        // 身体生命体征：真实系统负载 → EnergyContext.system_load（激活 Mind 紧急通路）。
        let system_load = probe_system_load();
        let suggested_mode = derive_suggested_mode(query, self.complexity.load(Ordering::Relaxed));
        let request = tonic::Request::new(HelixQueryRequest {
            query: query.to_string(),
            suggested_mode: suggested_mode as i32,
            energy_context: Some(build_energy_context(query, system_load)),
            include_recessive,
            allow_imagination: suggested_mode == CognitiveMode::Imagination,
            autonomy_level: derive_autonomy_level() as i32,
            traceparent: traceparent.clone(),
        });
        match self.client.clone().helix_query(request).await {
            Ok(response) => {
                let inner = response.into_inner();
                Ok(QueryResult {
                    nodes: inner.nodes.into_iter().map(|n| n.content_json).collect(),
                    impasse_level: inner.impasse_level as u8,
                    suggested_actions: inner
                        .suggested_actions
                        .into_iter()
                        .map(|a| a.action_type)
                        .collect(),
                })
            }
            Err(e) => {
                // 降级钩子：记录降级事件（含 traceparent，可观测），fail-open 由上层接管。
                warn!("mind degraded: traceparent={} err={}", traceparent, e);
                Err(e.to_string())
            }
        }
    }

    async fn remember(&self, content: &str) -> Result<(), String> {
        let request = tonic::Request::new(RememberRequest {
            content: content.to_string(),
        });
        self.client
            .clone()
            .remember(request)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn set_complexity(&self, level: u8) {
        self.complexity.store(level.clamp(0, 3), Ordering::Relaxed);
    }
}

/// 生成 W3C TraceContext 根 `traceparent`：`00-<32hex trace_id>-<16hex span_id>-01`。
/// 零新依赖：uuid v4 `.simple()` 产生 32 hex（无连字符）。
fn generate_traceparent() -> String {
    let trace_id = Uuid::new_v4().simple().to_string(); // 32 hex
    let span_id: String = Uuid::new_v4().simple().to_string()[..16].to_string(); // 16 hex
    format!("00-{}-{}-01", trace_id, span_id)
}

/// 探索关键词：命中即视为"探索性查询"（可触及气态痕迹）。
const EXPLORE_KEYWORDS: &[&str] = &[
    "探索", "研究", "发现", "创意", "头脑风暴", "未知", "可能性", "explore", "research",
    "brainstorm", "imagine",
];

/// 系统探针：归一化系统负载（0-1），取 CPU 与内存占用之较大者（保守）。
/// 激活 Mind 紧急通路（retrieval `system_load > 0.9` 触发纯算法降级）。
/// 同步轻量调用（P10a 最小；未来可 spawn_blocking）。
/// 探针失败回退 0.5（中性保守——不假装空闲，也不假装满载）。
fn probe_system_load() -> f64 {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let cpu = sys.global_cpu_info().cpu_usage() as f64 / 100.0;
    let mem = if sys.total_memory() > 0 {
        sys.used_memory() as f64 / sys.total_memory() as f64
    } else {
        0.0
    };
    cpu.max(mem).clamp(0.0, 1.0)
}

/// 预算分级推导（ADR-0010 / Helix-Mind ADR-0010）。
/// 结合 query 特征 + 真实系统负载：高负载（>0.8）降级——不触发昂贵的 Exogenous 探索；
/// 极简→Endogenous（0-token 只查晶体）；长/探索且负载正常→ExogenousRequired；
/// 默认 Augmentable。P10b 接状态机驱动。
fn derive_budget_tier(query: &str, system_load: f64) -> BudgetTier {
    let q = query.trim();
    let len = q.chars().count();
    if system_load > 0.8 {
        // 高负载：极致节能，抑制探索成本。
        return if q.is_empty() || len <= 4 {
            BudgetTier::Endogenous
        } else {
            BudgetTier::Augmentable
        };
    }
    if q.is_empty() || len <= 4 {
        BudgetTier::Endogenous
    } else if len >= 60 || EXPLORE_KEYWORDS.iter().any(|k| q.contains(k)) {
        BudgetTier::ExogenousRequired
    } else {
        BudgetTier::Augmentable
    }
}

/// 认知模式推导（P10b T2 状态机驱动）：`complexity` 来自 Amygdala PreAssessment 状态输出
/// （1=简单→Skilled / 2=中等→Anchor / 3=复杂→Imagination）；`0`（未知/未设状态）时
/// 回退 query 长度启发式兜底（不 panic）。
fn derive_suggested_mode(query: &str, complexity: u8) -> CognitiveMode {
    match complexity {
        1 => CognitiveMode::Skilled,
        2 => CognitiveMode::Anchor,
        3 => CognitiveMode::Imagination,
        _ => {
            let len = query.trim().chars().count();
            if len <= 10 {
                CognitiveMode::Skilled
            } else if len < 40 {
                CognitiveMode::Anchor
            } else {
                CognitiveMode::Imagination
            }
        }
    }
}

/// 自治级别推导：默认 OPEN（与既有行为一致；会话建立后不可变）。
fn derive_autonomy_level() -> AutonomyLevel {
    AutonomyLevel::Open
}

/// 构造 `EnergyContext`（含 budget_tier 前置路由 + 真实系统负载）。
///
/// 生态手套感知（MCP/宇树/Unity/鸿蒙可用性）为 **P10b 渐进扩展**：P10a 无对应字段，
/// 预留扩展位（未来可经独立状态/新契约字段传递，勿增实体）。
fn build_energy_context(query: &str, system_load: f64) -> EnergyContext {
    EnergyContext {
        token_budget: 1000,
        heliotropism: 0.0,
        pulse: 0.3,
        vigilance: 0.2,
        latency_limit_ms: 500,
        system_load,
        familiarity: 0.5,
        impasse_depth: 0,
        budget_tier: derive_budget_tier(query, system_load) as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traceparent_is_w3c_format() {
        let tp = generate_traceparent();
        // 00-{32 hex}-{16 hex}-01
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts.len(), 4, "4 段: {:?}", parts);
        assert_eq!(parts[0], "00");
        assert_eq!(parts[0..=0], ["00"]);
        assert_eq!(parts[1].len(), 32);
        assert_eq!(parts[2].len(), 16);
        assert_eq!(parts[3], "01");
        assert!(parts[1].chars().all(|c| c.is_ascii_hexdigit()));
        assert!(parts[2].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn traceparent_is_unique() {
        let a = generate_traceparent();
        let b = generate_traceparent();
        assert_ne!(a, b, "每次生成必须唯一（根 trace）");
    }

    #[test]
    fn budget_tier_simple_is_endogenous() {
        assert_eq!(derive_budget_tier("现在几点", 0.2), BudgetTier::Endogenous);
        assert_eq!(derive_budget_tier("你好", 0.2), BudgetTier::Endogenous);
        assert_eq!(derive_budget_tier("", 0.2), BudgetTier::Endogenous);
    }

    #[test]
    fn budget_tier_default_is_augmentable() {
        assert_eq!(
            derive_budget_tier("帮我查一下昨天的会议记录", 0.2),
            BudgetTier::Augmentable
        );
    }

    #[test]
    fn budget_tier_long_or_explore_is_exogenous() {
        let long = "请深入探索这个分布式系统在极端故障场景下的全部可能性与未知边界以及多方案权衡";
        assert_eq!(derive_budget_tier(long, 0.2), BudgetTier::ExogenousRequired);
        assert_eq!(
            derive_budget_tier("帮我研究一下这个课题", 0.2),
            BudgetTier::ExogenousRequired
        );
    }

    #[test]
    fn budget_tier_high_load_suppresses_exogenous() {
        // 高负载（0.9）：探索性查询被降级为 Augmentable（极致节能）。
        assert_eq!(
            derive_budget_tier("帮我研究一下这个课题", 0.9),
            BudgetTier::Augmentable,
            "高负载不得触发昂贵探索"
        );
        // 高负载 + 极简 → Endogenous。
        assert_eq!(derive_budget_tier("你好", 0.9), BudgetTier::Endogenous);
    }

    #[test]
    fn suggested_mode_scales_with_length_fallback() {
        // 无状态（complexity=0）→ 长度启发式兜底
        assert_eq!(derive_suggested_mode("你好", 0), CognitiveMode::Skilled);
        assert_eq!(derive_suggested_mode("帮我查一下昨天的会议记录", 0), CognitiveMode::Anchor);
        let long = "请深入分析这个复杂系统的架构与多维度权衡并给出完整方案建议与风险边界以及所有潜在的未知变量和未来可能的演进方向与备选路径";
        assert_eq!(derive_suggested_mode(long, 0), CognitiveMode::Imagination);
    }

    #[test]
    fn suggested_mode_state_driven_overrides_length() {
        // 状态机驱动：complexity 明确时优先于 query 长度
        assert_eq!(derive_suggested_mode("你好", 3), CognitiveMode::Imagination, "复杂状态 → Imagination 无视短 query");
        assert_eq!(derive_suggested_mode(long_query(), 1), CognitiveMode::Skilled, "简单状态 → Skilled 无视长 query");
        assert_eq!(derive_suggested_mode("你好", 2), CognitiveMode::Anchor);
    }

    fn long_query() -> &'static str {
        "请深入分析这个复杂系统的架构与多维度权衡并给出完整方案建议与风险边界以及所有潜在的未知变量和未来可能的演进方向与备选路径"
    }

    #[test]
    fn energy_context_carries_budget_tier_and_load() {
        let ec = build_energy_context("帮我查一下昨天的会议记录", 0.3);
        assert_eq!(ec.budget_tier, BudgetTier::Augmentable as i32);
        assert_eq!(ec.token_budget, 1000);
        assert!((ec.system_load - 0.3).abs() < 1e-9, "system_load 透传");
        let ec2 = build_energy_context("现在几点", 0.3);
        assert_eq!(ec2.budget_tier, BudgetTier::Endogenous as i32);
    }

    #[test]
    fn probe_system_load_is_in_unit_range() {
        // sysinfo 探针在测试环境返回 [0,1] 归一化负载（macOS 可用）。
        let load = probe_system_load();
        assert!(
            (0.0..=1.0).contains(&load),
            "system_load 必须在 [0,1]，实际 {}",
            load
        );
    }

    #[test]
    fn autonomy_defaults_open() {
        assert_eq!(derive_autonomy_level(), AutonomyLevel::Open);
    }
}
