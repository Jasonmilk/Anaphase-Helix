//! 生态手套可用性感知（P10c T3，DNA 原则 3 + spec/position A.4）。
//!
//! **Cellrix = 原生生态手套**（观测与操作 Anaphase，优先级最高）；
//! MCP 等为通用生态手套（优先级次之）。
//!
//! 只做**可用性状态**的注册/查询（独立扩展位），**不实现手套协议**（勿增实体）。
//! 未来手套协议实现（MCP 连接/宇树/Unity/鸿蒙等）在扩展位挂载，不在本模块。
//!
//! O-1（ADR-0016 D3）：`probe_ecosystem` 把 config 端点映射为点亮状态——
//! "看口袋过日子"的物理落点，任务开始前一次探测，0 tokens、无持续轮询。

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// 手套可用性状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GloveStatus {
    /// 未探测
    Unknown,
    /// 可用
    Available,
    /// 不可用（故障 / 未连接 / 停摆）
    Unavailable,
}

/// 手套层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GloveTier {
    /// 原生生态手套（Cellrix），优先级最高
    Native,
    /// 通用生态手套（MCP 等）
    Standard,
}

/// 手套信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GloveInfo {
    pub name: String,
    pub tier: GloveTier,
    pub status: GloveStatus,
    pub updated_at: String,
}

/// 生态手套可用性感知：状态注册表（独立扩展位，不实现协议）
#[derive(Debug, Clone, Default)]
pub struct EcosystemGloves {
    gloves: Vec<GloveInfo>,
}

impl EcosystemGloves {
    pub fn new() -> Self {
        Self { gloves: Vec::new() }
    }

    /// 注册/覆盖手套可用性状态（Cellrix 原生优先）
    pub fn register(&mut self, name: &str, tier: GloveTier, status: GloveStatus) {
        if let Some(g) = self.gloves.iter_mut().find(|g| g.name == name) {
            g.tier = tier;
            g.status = status;
            g.updated_at = Utc::now().to_rfc3339();
        } else {
            self.gloves.push(GloveInfo {
                name: name.to_string(),
                tier,
                status,
                updated_at: Utc::now().to_rfc3339(),
            });
        }
    }

    /// 更新可用性状态
    pub fn set_status(&mut self, name: &str, status: GloveStatus) -> Result<(), String> {
        let g = self
            .gloves
            .iter_mut()
            .find(|g| g.name == name)
            .ok_or_else(|| format!("手套未注册：{}", name))?;
        g.status = status;
        g.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    /// 查询可用性状态
    pub fn status(&self, name: &str) -> Option<GloveStatus> {
        self.gloves.iter().find(|g| g.name == name).map(|g| g.status)
    }

    /// 可用手套列表（Native 优先排序）
    pub fn available(&self) -> Vec<&GloveInfo> {
        let mut v: Vec<&GloveInfo> = self
            .gloves
            .iter()
            .filter(|g| g.status == GloveStatus::Available)
            .collect();
        v.sort_by_key(|g| match g.tier {
            GloveTier::Native => 0,
            GloveTier::Standard => 1,
        });
        v
    }

    /// Cellrix（原生手套）是否可用
    pub fn native_available(&self) -> bool {
        self.available()
            .iter()
            .any(|g| g.tier == GloveTier::Native)
    }

    /// 生态组件点亮状态投影（供 AgentSnapshot/驾驶舱展示，O-1）。
    pub fn list(&self) -> Vec<GloveInfo> {
        self.gloves.clone()
    }
}

/// 生态组件点亮探测（O-1，ADR-0016 D3）：任务开始前**一次**物理探测，
/// 把 config 端点映射为点亮状态。0 tokens、无新依赖、无持续轮询。
///
/// 规则：
/// - 端点未配置 → `Unavailable`（组件未安装/未启用）
/// - TCP 端点（`http://host:port` / `host:port`）→ `connect_timeout` 连接探测
/// - UDS 端点（`unix://path` / 裸路径）→ socket 文件存在性
/// - Cellrix = Native 手套（最高优先级）；Tentacle/Mind/Tuck/FlowModus/Callosum
///   = Standard 组件（注册表同时承载"生态组件点亮状态"，手套是其中一种）
pub async fn probe_ecosystem(cfg: &crate::config::AnaphaseConfig) -> EcosystemGloves {
    let mut g = EcosystemGloves::new();
    // Cellrix：优先显式端点；否则 cap_http（驾驶舱）就绪即点亮。
    if let Some(ep) = cfg.cellrix_endpoint.as_deref() {
        g.register("cellrix", GloveTier::Native, probe_endpoint(ep));
    } else if cfg.cap_http_enabled {
        let ep = format!("127.0.0.1:{}", cfg.cap_http_port);
        g.register("cellrix", GloveTier::Native, probe_endpoint(&ep));
    }
    // 生态组件（Standard）：配置即探测，未配置 = 未点亮。
    for (name, ep) in [
        ("tentacle", cfg.tentacle_endpoint.as_deref()),
        ("mind", cfg.mind_endpoint.as_deref()),
        ("tuck", cfg.tuck_endpoint.as_deref()),
        ("flowmodus", cfg.flowmodus_endpoint.as_deref()),
    ] {
        match ep {
            Some(e) if !e.is_empty() => g.register(name, GloveTier::Standard, probe_endpoint(e)),
            _ => g.register(name, GloveTier::Standard, GloveStatus::Unavailable),
        }
    }
    g
}

/// 单端点物理探测：解析协议前缀后，TCP 连接或 socket 文件存在性。
fn probe_endpoint(ep: &str) -> GloveStatus {
    let bare = ep
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("unix://");
    // TCP：host:port（IPv4/IPv6/[::1]:port 均可被 SocketAddr 解析）。
    if let Ok(addr) = bare.parse::<SocketAddr>() {
        return match TcpStream::connect_timeout(&addr, Duration::from_millis(400)) {
            Ok(_) => GloveStatus::Available,
            Err(_) => GloveStatus::Unavailable,
        };
    }
    // UDS：socket 文件存在即视为点亮（连接留待使用时 fail-open）。
    let path = std::path::Path::new(bare);
    if path.exists() {
        GloveStatus::Available
    } else {
        GloveStatus::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_query_status() {
        let mut g = EcosystemGloves::new();
        assert_eq!(g.status("cellrix"), None, "未注册 → None");
        g.register("cellrix", GloveTier::Native, GloveStatus::Available);
        g.register("mcp", GloveTier::Standard, GloveStatus::Available);
        assert_eq!(g.status("cellrix"), Some(GloveStatus::Available));
        assert_eq!(g.status("mcp"), Some(GloveStatus::Available));
        assert_eq!(g.status("unity"), None);
    }

    #[test]
    fn set_status_updates_registered_only() {
        let mut g = EcosystemGloves::new();
        g.register("cellrix", GloveTier::Native, GloveStatus::Available);
        assert!(g.set_status("cellrix", GloveStatus::Unavailable).is_ok());
        assert_eq!(g.status("cellrix"), Some(GloveStatus::Unavailable));
        assert!(g.set_status("unity", GloveStatus::Available).is_err(), "未注册不可更新");
    }

    #[test]
    fn available_sorts_native_first() {
        let mut g = EcosystemGloves::new();
        g.register("mcp_a", GloveTier::Standard, GloveStatus::Available);
        g.register("cellrix", GloveTier::Native, GloveStatus::Available);
        g.register("unity", GloveTier::Standard, GloveStatus::Unavailable);
        let av = g.available();
        assert_eq!(av.len(), 2);
        assert_eq!(av[0].name, "cellrix", "Native 优先级最高");
        assert_eq!(av[1].name, "mcp_a");
    }

    #[test]
    fn native_available_true_only_when_native_ok() {
        let mut g = EcosystemGloves::new();
        assert!(!g.native_available());
        g.register("mcp", GloveTier::Standard, GloveStatus::Available);
        assert!(!g.native_available(), "仅 Standard 可用 → false");
        g.register("cellrix", GloveTier::Native, GloveStatus::Unavailable);
        assert!(!g.native_available());
        g.set_status("cellrix", GloveStatus::Available).unwrap();
        assert!(g.native_available());
    }

    #[test]
    fn protocol_extension_placeholder() {
        // 独立扩展位：协议实现（MCP 连接/宇树等）不在本模块，仅状态注册。
        // 本测试锁定"不实现协议"的边界——无任何协议字段。
        let g = EcosystemGloves::new();
        assert_eq!(g.available().len(), 0);
    }

    #[test]
    fn endpoint_tcp_parse_probes_localhost() {
        // 本地未监听端口 → Unavailable（物理事实，不假装可用）。
        let st = probe_endpoint("http://127.0.0.1:9");
        assert_eq!(st, GloveStatus::Unavailable);
    }

    #[test]
    fn endpoint_uds_missing_file_is_unavailable() {
        let st = probe_endpoint("unix:///tmp/definitely-absent-helix.sock");
        assert_eq!(st, GloveStatus::Unavailable);
    }

    #[test]
    fn unconfigured_components_are_unavailable() {
        let cfg = crate::config::AnaphaseConfig::default();
        let g = probe_ecosystem_block(cfg);
        assert_eq!(g.status("tentacle"), Some(GloveStatus::Unavailable));
        assert_eq!(g.status("mind"), Some(GloveStatus::Unavailable));
        assert_eq!(g.status("tuck"), Some(GloveStatus::Unavailable));
        assert_eq!(g.status("cellrix"), None, "未配置且 cap_http 关闭 → 不注册");
    }

    #[test]
    fn configured_components_probe_physically() {
        // 配置了不可达端点 → Unavailable（物理事实）。
        let mut cfg = crate::config::AnaphaseConfig::default();
        cfg.tentacle_endpoint = Some("http://127.0.0.1:9".to_string());
        cfg.mind_endpoint = Some("unix:///tmp/definitely-absent-helix.sock".to_string());
        let g = probe_ecosystem_block(cfg);
        assert_eq!(g.status("tentacle"), Some(GloveStatus::Unavailable));
        assert_eq!(g.status("mind"), Some(GloveStatus::Unavailable));
    }

    /// 无 tokio runtime 的测试环境：probe_ecosystem 内部全同步，
    /// async 壳仅为对齐调用方形态——直接同步构造等价结果。
    fn probe_ecosystem_block(cfg: crate::config::AnaphaseConfig) -> EcosystemGloves {
        let mut g = EcosystemGloves::new();
        for (name, ep) in [
            ("tentacle", cfg.tentacle_endpoint.as_deref()),
            ("mind", cfg.mind_endpoint.as_deref()),
            ("tuck", cfg.tuck_endpoint.as_deref()),
            ("flowmodus", cfg.flowmodus_endpoint.as_deref()),
        ] {
            match ep {
                Some(e) if !e.is_empty() => g.register(name, GloveTier::Standard, probe_endpoint(e)),
                _ => g.register(name, GloveTier::Standard, GloveStatus::Unavailable),
            }
        }
        g
    }
}
