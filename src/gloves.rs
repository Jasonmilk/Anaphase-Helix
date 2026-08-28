//! 生态手套可用性感知（P10c T3，DNA 原则 3 + spec/position A.4）。
//!
//! **Cellrix = 原生生态手套**（观测与操作 Anaphase，优先级最高）；
//! MCP 等为通用生态手套（优先级次之）。
//!
//! 只做**可用性状态**的注册/查询（独立扩展位），**不实现手套协议**（勿增实体）。
//! 未来手套协议实现（MCP 连接/宇树/Unity/鸿蒙等）在扩展位挂载，不在本模块。

use chrono::Utc;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GloveInfo {
    pub name: String,
    pub tier: GloveTier,
    pub status: GloveStatus,
    pub updated_at: String,
}

/// 生态手套可用性感知：状态注册表（独立扩展位，不实现协议）
#[derive(Debug, Default)]
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
        assert!(g.native_available());
    }

    #[test]
    fn set_status_update() {
        let mut g = EcosystemGloves::new();
        g.register("cellrix", GloveTier::Native, GloveStatus::Available);
        g.set_status("cellrix", GloveStatus::Unavailable).unwrap();
        assert_eq!(g.status("cellrix"), Some(GloveStatus::Unavailable));
        assert!(!g.native_available(), "Cellrix 停摆 → 原生不可用");
        assert!(g.set_status("ghost", GloveStatus::Available).is_err());
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
    fn protocol_extension_placeholder() {
        // 独立扩展位：协议实现（MCP 连接/宇树等）不在本模块，仅状态注册。
        // 本测试锁定"不实现协议"的边界——无任何协议字段。
        let g = EcosystemGloves::new();
        assert_eq!(g.available().len(), 0);
    }
}
