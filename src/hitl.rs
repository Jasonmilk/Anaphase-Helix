//! HITL 人在回路审批通道（P10b T3，DNA 原则 4）。
//!
//! 三层闸门之一（执行闸）：工具审计（入库门）→ **HITL（执行闸）** → Tuck（边缘物理闸）。
//! HITL 管"这次动作能不能执行"：高风险动作（写操作/网络请求/凭证使用）必须经人类确认，
//! 未经确认被物理拦截。低风险动作零额外延迟（直接放行）。
//!
//! 默认 **fail-closed**：无人类确认通道时，高风险动作拦截（HITL 物理语义，安全优先）。

use std::sync::Arc;

/// 人类确认回调：`(command, args) -> Ok(true)=确认放行 / Ok(false)=拒绝 / Err=无通道`
pub type ApproveFn = Arc<dyn Fn(&str, &[String]) -> Result<bool, String> + Send + Sync>;

pub struct HITLApprover {
    approver: ApproveFn,
}

impl Default for HITLApprover {
    fn default() -> Self {
        // fail-closed：无确认通道时，高风险动作拦截（Err = 通道缺失）
        Self {
            approver: Arc::new(|_cmd, _args| {
                Err("No HITL confirmation channel configured".to_string())
            }),
        }
    }
}

impl HITLApprover {
    pub fn new(approver: ApproveFn) -> Self {
        Self { approver }
    }

    /// 高风险动作判定：写操作 / 网络请求 / 凭证使用
    pub fn is_high_risk(command: &str) -> bool {
        const WRITE: &[&str] = &[
            "rm", "mv", "cp", "mkdir", "touch", "truncate", "dd", "shred", "write", "delete",
            "remove", "unlink",
        ];
        const NETWORK: &[&str] = &[
            "curl", "wget", "nc", "ncat", "ssh", "scp", "sftp", "http", "https", "fetch",
            "post", "send",
        ];
        const CREDENTIAL: &[&str] = &[
            "token", "key", "secret", "password", "cookie", "credential", "api_key", "bearer",
        ];
        let cmd = command.to_lowercase();
        let tokens: Vec<String> = cmd
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        for t in &tokens {
            if WRITE.contains(&t.as_str())
                || NETWORK.contains(&t.as_str())
                || CREDENTIAL.contains(&t.as_str())
            {
                return true;
            }
        }
        // 凭证子串匹配（如 my_api_key / send_credentials）
        let compact = cmd.replace(['_', '-'], "");
        CREDENTIAL.iter().any(|k| compact.contains(k))
    }

    /// HITL 执行闸：低风险 → 直接放行；高风险 → 请求人类确认
    pub fn check_approval(&self, command: &str, args: &[String]) -> Result<bool, String> {
        if !Self::is_high_risk(command) {
            return Ok(true);
        }
        (self.approver)(command, args)
    }
}

/// 人类答复的**闭集**（参照 DSH 的 `ApprovalOutcome`，四值同为闭集）。
///
/// 布尔会把"人说了不可以"和"**没有任何人能回答**"折叠成一个值，而调用方很快
/// 又把它折成同一个 `TransitionCondition` —— 这就是 K-033/K-114 的"缺席被当成
/// 通过／被当成拒绝"。闭集让"缺席"成为一等事实：可记录、可断言、可显示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalOutcome {
    /// 人明确批准**这一次**。唯一授权，且不自动延续到下一个动作。
    AllowedOnce,
    /// 人明确拒绝。
    Rejected,
    /// 提问被撤回（超时、回合中止）。**不是**批准。
    Cancelled,
    /// 没有应答者或应答者失败。fail-closed 的落点，**不是**批准。
    Unavailable,
}

impl ApprovalOutcome {
    /// 四值里**只有** `AllowedOnce` 允许动作发生 —— A5 的安全底线。
    pub fn allows_execution(self) -> bool {
        matches!(self, ApprovalOutcome::AllowedOnce)
    }
}

/// 等待的**呈现**方式，不承载安全语义（三态/四值/fail-closed/审计成对都与它无关）。
///
/// 同一句需求的三个参数，不是三种机制：挂起始终是 async suspend（不轮询、不占线程）；
/// `approval/asked` 始终**先落盘再等待**（DSH 同法），所以"先发送完再等"是默认行为，
/// 那条记录本身就是"告知"；`Announce` 只在长等待时额外广播一条状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitMode {
    /// 等了，不额外交代（预期瞬时答复）。
    Silent,
    /// 等了，并让等待对外可见（长等待）。
    Announce,
    /// 先把已推进的部分发完，再等。
    AwaitingHuman,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_risk_detection() {
        // 写操作
        assert!(HITLApprover::is_high_risk("rm -rf /data"));
        assert!(HITLApprover::is_high_risk("mv file.txt /tmp"));
        // 网络请求
        assert!(HITLApprover::is_high_risk("curl https://example.com"));
        assert!(HITLApprover::is_high_risk("wget http://x"));
        // 凭证使用
        assert!(HITLApprover::is_high_risk("ssh deploy@host"));
        assert!(HITLApprover::is_high_risk("use_token"));
        // 低风险
        assert!(!HITLApprover::is_high_risk("echo hello"));
        assert!(!HITLApprover::is_high_risk("perceive"));
    }

    #[test]
    fn low_risk_passes_without_approver() {
        // 低风险 → 零延迟放行（即使无确认通道）
        let h = HITLApprover::default();
        assert_eq!(h.check_approval("echo", &[]).unwrap(), true);
    }

    #[test]
    fn high_risk_fail_closed_without_channel() {
        // 高风险 + 无确认通道 → fail-closed 拦截（Err）
        let h = HITLApprover::default();
        assert!(h.check_approval("rm -rf /data", &[]).is_err());
    }

    #[test]
    fn high_risk_approve_deny() {
        let approve = HITLApprover::new(Arc::new(|_c, _a| Ok(true)));
        assert_eq!(approve.check_approval("curl http://x", &[]).unwrap(), true);

        let deny = HITLApprover::new(Arc::new(|_c, _a| Ok(false)));
        assert_eq!(deny.check_approval("curl http://x", &[]).unwrap(), false);
    }
}
