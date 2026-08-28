//! 纪元生命周期（P10c T1，DNA 原则 3）：强制苏醒 + 认知脱水。
//!
//! 纪元（Epoch）= Anaphase 每次唤醒。跨纪元认知传承：
//! - 强制苏醒 `wake_up()`：读取上一纪元 `session_notes.json` 认知脱水简报（认知重载）
//! - 认知脱水 `dehydrate()`：当前纪元结束前压缩历史为简报，供下一纪元加载
//!
//! 工作态归 Anaphase（DNA 原则 3）：简报是 Anaphase 私有工作态，
//! 非 L2/L3 长期记忆（归 Helix-Mind），不触碰 L3 不可篡改铁律。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// 跨纪元认知重载：醒来时知道"前世走到哪一步"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Awakening {
    /// 是否存在上一纪元的脱水简报
    pub has_history: bool,
    /// 简报内容（上一纪元认知脱水）
    pub briefing: String,
}

/// 认知脱水简报（纪元结束前持久化）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dehydration {
    pub epoch_id: String,
    pub briefing: String,
    pub history_len: usize,
    pub created_at: String,
}

/// 会话笔记：强制苏醒 + 认知脱水 的持久化载体
pub struct SessionNotes {
    path: PathBuf,
}

impl SessionNotes {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 强制苏醒：读取上一纪元认知脱水简报（跨纪元认知重载）
    pub fn wake_up(&self) -> Result<Awakening, String> {
        if !self.path.exists() {
            return Ok(Awakening {
                has_history: false,
                briefing: String::new(),
            });
        }
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("wake_up read failed: {e}"))?;
        match serde_json::from_str::<Dehydration>(&content) {
            Ok(d) => Ok(Awakening {
                has_history: true,
                briefing: d.briefing,
            }),
            Err(_) => Ok(Awakening {
                has_history: true,
                briefing: "(session notes malformed — 保留原文件，等待检修)".to_string(),
            }),
        }
    }

    /// 认知脱水：压缩当前纪元历史并持久化为简报（跨纪元上下文压缩）
    pub fn dehydrate(&self, history: &[String]) -> Result<Dehydration, String> {
        let briefing = compress_briefing(history);
        let d = Dehydration {
            epoch_id: Utc::now().timestamp().to_string(),
            briefing,
            history_len: history.len(),
            created_at: Utc::now().to_rfc3339(),
        };
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create session dir failed: {e}"))?;
        }
        let json = serde_json::to_string_pretty(&d)
            .map_err(|e| format!("serialize failed: {e}"))?;
        std::fs::write(&self.path, json).map_err(|e| format!("dehydrate write failed: {e}"))?;
        Ok(d)
    }
}

/// 确定性认知脱水压缩：去重 + 拼接 + 截断（0 Token，无 LLM）。
/// LLM 压缩端口（P 阶段预留）：当前以确定性压缩满足跨纪元重载；
/// 未来可接轻量 LLM 提升简报密度（不影响存储契约）。
pub fn compress_briefing(history: &[String]) -> String {
    const MAX_CHARS: usize = 2000;
    let mut seen: HashSet<String> = HashSet::new();
    let mut lines: Vec<String> = Vec::new();
    for h in history {
        let t = h.trim();
        if t.is_empty() || !seen.insert(t.to_string()) {
            continue;
        }
        lines.push(t.to_string());
    }
    let joined = lines.join("\n");
    if joined.chars().count() > MAX_CHARS {
        let mut out: String = joined.chars().take(MAX_CHARS).collect();
        out.push_str("\n...[briefing truncated]");
        out
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_notes() -> (SessionNotes, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "anaphase_lifecycle_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let path = dir.join("nested").join("session_notes.json");
        (SessionNotes::new(path.clone()), path)
    }

    #[test]
    fn wake_up_no_history() {
        let (notes, _path) = temp_notes();
        let a = notes.wake_up().unwrap();
        assert!(!a.has_history);
        assert!(a.briefing.is_empty());
    }

    #[test]
    fn dehydrate_then_wake_up_roundtrip() {
        let (notes, path) = temp_notes();
        let history = vec!["user: 帮我查资料".to_string(), "assistant: 好的".to_string()];
        let d = notes.dehydrate(&history).unwrap();
        assert_eq!(d.history_len, 2);
        assert!(d.briefing.contains("帮我查资料"));

        // 强制苏醒读回
        let a = notes.wake_up().unwrap();
        assert!(a.has_history);
        assert_eq!(a.briefing, d.briefing);

        // 清理
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn compress_dedup_and_empty() {
        // 去重
        let h = vec![
            "同一行".to_string(),
            "同一行".to_string(),
            "  " .to_string(),
            "另一行".to_string(),
        ];
        let b = compress_briefing(&h);
        assert_eq!(b.matches("同一行").count(), 1);
        assert!(b.contains("另一行"));
        // 空
        assert!(compress_briefing(&[]).is_empty());
    }

    #[test]
    fn compress_truncates_long_history() {
        let long_line = "x".repeat(100);
        let h: Vec<String> = (0..50).map(|i| format!("{}{}", long_line, i)).collect();
        let b = compress_briefing(&h);
        assert!(b.contains("truncated"));
        assert!(b.chars().count() <= 2100);
    }

    #[test]
    fn dehydrate_creates_nested_dirs() {
        let (notes, path) = temp_notes();
        let parent = path.parent().unwrap();
        assert!(!parent.exists());
        notes.dehydrate(&["a".to_string()]).unwrap();
        assert!(parent.exists(), "嵌套目录应自动创建");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(parent);
    }
}
