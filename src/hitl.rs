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
    /// THE DECISION SURFACE IS THE CALL, NOT THE NAME (ADR-0048 §213).
    ///
    /// Measured: judging the tool name alone intervenes on **0/12** realistic dangerous calls —
    /// the dangerous content lives in the arguments while the name is neutral ("run_command",
    /// "shell", "terminal", "executor", "dispatch"…). Worse, the name is produced by the party being
    /// monitored (`contract/mod.rs:332: tool: name.to_string()` parses the LLM's own
    /// `ReasoningSignal`), so a name list is an adaptive adversary's easiest thing to defeat
    /// (measured interception for a 110-name list: 11% at 10³ names, 0.11% at 10⁵).
    ///
    /// `args_json` is the SAME serialization the `Some(gate)` path already passes as
    /// `GateCheck::args_json`, so the two paths finally judge the same thing.
    pub fn is_high_risk_call(tool: &str, args_json: &str) -> bool {
        /* THE SURFACE IS CAPABILITY, NOT TEXT (ADR-0048 §215), and §213's "tool ∨ args text" was
         * only half right — measured, scanning the args as free text gives:
         *   · false positives 11/12 (91.7%): `{"key":"theme"}` hits CREDENTIAL's `key`;
         *     `{"query":"how to remove a stain"}` hits WRITE's `remove`; `{"path":"…/.ssh/id_rsa.pub"}`
         *     hits NETWORK's `ssh`; `translate {"text":"please delete the draft"}` hits `delete`;
         *   · misses 2/2 on the ELABORATE dangerous forms — `os.system(base64.b64decode(...))` and
         *     `chr(114)+chr(109)` contain no list token at all.
         *   ⇒ I = 0.0047 bits: "blocked" barely carries "dangerous".
         * The reason is a category error: for `read_file`/`search`/`translate` the args are DATA
         * (a path, a query, a sentence); treating data as a command is exactly the conflation this
         * repo keeps paying for. For an executor the args are instructions — and then they are
         * UNDECIDABLE, which is a reason to refuse that does NOT depend on reading them.
         *
         * So: a command-bearing FIELD is what we judge, never its content. The rule is stable under
         * every textual disguise, because it never looks at the text. */
        /* The NAME axis is kept, but narrowed to the EXECUTION capability only. Measured: the full
         * token classifier fires on `send_message` (token `send` ∈ NETWORK), which is a message tool,
         * not a command runner — the name axis is over-broad in exactly the way §213-D described
         * (and the name is chosen by the LLM). Executors are matched by name because their NAME IS
         * their capability; everything else is decided by the shape of the call. */
        if Self::is_executor_name(tool) || Self::is_dangerous_name(tool) {
            return true;
        }
        Self::carries_command(args_json)
    }

    /// THE REFUSAL MUST NOT NAME ITS TRIGGER (ADR-0048 §216 ⑤): naming the field or tool that
    /// fired teaches an adaptive adversary exactly which name to change next. This states the
    /// capability position instead — "you did not declare a class", never "you wrote `code`".
    pub fn capability_undeclared_reason() -> &'static str {
        "capability-undeclared: this call declares no capability class, and an undeclared \
         capability is not permission (B7). Declare one in the tool registry to allow it."
    }

    /// UN-AMBIGUOUS NAMES ONLY (ADR-0048 §220 ③). §216 narrowed the name axis to EXECUTION and, in
    /// doing so, silently dropped WRITE/NETWORK/CREDENTIAL *by name*: measured, after that change
    /// `is_high_risk_call("rm", {"path":"/data"})` returned **false** — a tool literally called `rm`
    /// was allowed by name. The false positives §216 removed came from COMPOUND names and GENERIC
    /// FIELD names, so the repair keeps those out while restoring the unambiguous verbs: a name is
    /// dangerous if it IS one of them, or if EVERY word in it is dangerous (so `code_review`,
    /// `dry_run`, `send_message`, `run_query` stay allowed — those were the measured FPs).
    pub fn is_dangerous_name(tool: &str) -> bool {
        const DESTRUCTIVE: &[&str] = &[
            "rm", "dd", "shred", "truncate", "unlink", "mkfs", "fdisk", "format",
        ];
        const NETWORK: &[&str] = &[
            "curl", "wget", "nc", "ncat", "netcat", "ssh", "scp", "sftp", "telnet", "ftp",
        ];
        const PRIVILEGE: &[&str] = &[
            "sudo", "su", "doas", "pkexec", "chmod", "chown", "chgrp", "mount", "umount", "kill",
            "killall", "pkill", "shutdown", "reboot", "halt", "systemctl", "launchctl", "crontab",
        ];
        /* THE OTHER GATE'S HALF (ADR-0048 §222). Measured: the ENGINE gate's list (`is_high_risk`)
         * and this pipeline gate's list shared only 79 words (Jaccard 0.577) — 30 words were blocked
         * by the engine and invisible here, and this module's own header says high-risk means
         * "write / network / credential". The pipeline gate scored **0/20** on bare names for those
         * three classes, so `delete_database {"target":"prod"}` and `read_secret {"name":"api_key"}`
         * reached Tentacle without confirmation on this path.
         * These are BARE-name words: a tool called `key` is a credential reader, while a FIELD called
         * `key` is not scanned by this predicate at all (that was §216's false-positive channel). */
        const WRITE_BARE: &[&str] = &[
            "write", "delete", "remove", "mv", "cp", "move", "copy", "mkdir", "touch", "rmdir",
        ];
        const NETWORK_BARE: &[&str] = &["http", "https", "fetch", "post", "send", "request", "upload"];
        const CREDENTIAL_BARE: &[&str] = &[
            "token", "key", "secret", "password", "passwd", "cookie", "credential", "credentials",
            "apikey", "bearer", "auth",
        ];
        let lower = tool.to_lowercase();
        let bare = lower.replace(['_', '-'], "");
        if DESTRUCTIVE.contains(&bare.as_str())
            || NETWORK.contains(&bare.as_str())
            || PRIVILEGE.contains(&bare.as_str())
            || WRITE_BARE.contains(&bare.as_str())
            || NETWORK_BARE.contains(&bare.as_str())
            || CREDENTIAL_BARE.contains(&bare.as_str())
        {
            return true;
        }
        let toks: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        !toks.is_empty()
            && toks.iter().all(|t| {
                DESTRUCTIVE.contains(t) || NETWORK.contains(t) || PRIVILEGE.contains(t)
            })
    }

    /// Is this tool itself a program runner? Matched on its own tokens against the EXECUTION
    /// capability list — the one place where the name really is the capability.
    pub fn is_executor_name(tool: &str) -> bool {
        const EXECUTION: &[&str] = &[
            "bash", "sh", "zsh", "fish", "ksh", "dash", "csh", "tcsh", "cmd", "powershell", "pwsh",
            "python", "python2", "python3", "py", "node", "nodejs", "deno", "bun", "perl", "ruby",
            "php", "lua", "julia", "awk", "sed", "eval", "exec", "execute", "executor", "xargs",
            "sudo", "su", "doas", "pkexec", "docker", "podman", "kubectl", "helm", "terraform",
            "ansible", "vagrant", "systemctl", "service", "launchctl", "crontab", "mount", "umount",
            "chmod", "chown", "chgrp", "pip", "pip3", "npm", "npx", "yarn", "pnpm", "cargo", "make",
            "cmake", "gradle", "mvn", "gem", "bundle", "apt", "brew", "yum", "dnf", "pacman",
            "shell", "terminal", "interpreter", "spawn", "process", "run", "runcommand", "invoke",
            "dispatch", "computeruse", "sandbox", "code", "script", "command",
        ];
        let lower = tool.to_lowercase();
        let bare = lower.replace(['_', '-'], "");
        if EXECUTION.contains(&bare.as_str()) {
            return true;
        }
        /* ALL tokens must be execution words (ADR-0048 §216). Measured with `any`: `dry_run` fires
         * (token `run`) although it is the one tool that does NOT run, and `code_review`,
         * `process_document`, `invoke_api`, `script_lint`, `shell_completion`, `run_query`,
         * `service_status` all fired on a single token. The judgement was inverted on `dry_run` —
         * a name is an executor only if every word in it is an execution word. */
        let toks: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        !toks.is_empty() && toks.iter().all(|t| EXECUTION.contains(t))
    }

    /// Does this call hand a *program* to something that could run it? Text-independent by design:
    /// an executor's args are undecidable, so the refusal cites the SHAPE of the call, not its words.
    /// Unparseable args ⇒ refuse (fail-closed: undecidable is not permission — the B7 rule).
    pub fn carries_command(args_json: &str) -> bool {
        /* EXACT, EXPLICIT COMMAND FIELDS ONLY (ADR-0048 §216). Measured false positives from the
         * previous matching (`ends_with`/`starts_with` + generic names + `_ => true` on any value):
         *   translate {"source":"en"} · llm_complete {"input":"…"} (an OpenAI STANDARD field) ·
         *   embeddings {"input":[…]} · form_submit {"action":"submit"} · render {"payload":{…}} ·
         *   analyze {"expression":"a+b"} · build {"run_id":42} · background_job {"run":false}
         * — the last two are the inverted case: `{"run":false}` means DO NOT RUN and it was read as
         * "run", because Bool/Number fell into `_ => true`. A key is a command field only if it NAMES
         * a program, exactly, and only a string/array value carries anything. */
        const CMD_FIELDS: &[&str] = &[
            "cmd", "command", "commandline", "script", "code", "stdin", "bin", "binary", "exec",
            "execute", "argv", "program", "shell", "eval", "snippet",
        ];
        let v: serde_json::Value = match serde_json::from_str(args_json) {
            Ok(v) => v,
            /* Undecidable ⇒ refuse. */
            Err(_) => return true,
        };
        match v {
            serde_json::Value::Object(map) => map.iter().any(|(k, val)| {
                let k = k.to_lowercase();
                let bare = k.replace(['_', '-'], "");
                if !CMD_FIELDS.iter().any(|f| &k == f || &bare == f) {
                    return false;
                }
                match val {
                    serde_json::Value::String(s) => !s.trim().is_empty(),
                    serde_json::Value::Array(a) => !a.is_empty(),
                    /* Bool / Number / Object carry no program: `{"run":false}` is a REFUSAL to run,
                     * and reading it as a command inverted the judgement. */
                    _ => false,
                }
            }),
            _ => false,
        }
    }

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
        /* THE CAPABILITY AXIS (ADR-0048 §212). Measured before this line existed: the list above
         * blocked `rm`/`curl`/`ssh` (name axis 28.3%) and let `bash`/`python3`/`sudo`/`docker`
         * through (capability axis **0.0%**) — and an interpreter can perform every blocked action.
         * Saltzer & Schroeder (1975) name the failure direction of any exclusion list:
         *   "Access decisions should be based on permission rather than exclusion… mechanisms that
         *    identify conditions under which access should be refused present the wrong psychological
         *    base"; and "a mistake in a mechanism that explicitly excludes access tends to fail by
         *    ALLOWING access… may go unnoticed in normal use."
         * So this is an INTERIM step, not the destination: the destination is an allow-list
         * (permission-based). Until then, anything that can run anything is high-risk by CAPABILITY,
         * regardless of how harmless its name looks. */
        const EXECUTION: &[&str] = &[
            /* shells */
            "bash", "sh", "zsh", "fish", "ksh", "dash", "csh", "tcsh", "cmd", "powershell", "pwsh",
            /* interpreters / evaluators */
            "python", "python2", "python3", "py", "node", "nodejs", "deno", "bun", "perl", "ruby",
            "php", "lua", "r", "julia", "awk", "sed", "eval", "exec", "source", "xargs", "env",
            /* privilege / orchestration / containers */
            "sudo", "su", "doas", "pkexec", "docker", "podman", "kubectl", "helm", "terraform",
            "ansible", "vagrant", "systemctl", "service", "launchctl", "crontab", "at", "mount",
            "umount", "chmod", "chown", "chgrp", "useradd", "usermod", "passwd",
            /* package managers / build tools (they run arbitrary post-install scripts) */
            "pip", "pip3", "npm", "npx", "yarn", "pnpm", "cargo", "go", "make", "cmake", "gradle",
            "maven", "mvn", "gem", "bundle", "apt", "apt-get", "brew", "yum", "dnf", "pacman",
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
                || EXECUTION.contains(&t.as_str())
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


    #[test]
    fn the_pipeline_gate_sees_the_write_network_and_credential_bare_names() {
        /* §222 measured 0/20 here: these were blocked by the ENGINE gate and invisible to this one. */
        let bare = [
            "write", "delete", "remove", "mv", "cp", "mkdir", "touch",
            "http", "https", "fetch", "post", "send",
            "token", "key", "secret", "password", "cookie", "credential", "api_key", "bearer",
        ];
        for t in bare {
            assert!(HITLApprover::is_high_risk_call(t, "{}"),
                "{t} IS a write/network/credential verb \u{21d2} the pipeline gate must see it");
        }
        /* And the §216 false-positive shapes must STILL be allowed: a compound name is not a verb, and
         * a field name is never scanned. */
        for (tool, args) in [
            ("code_review", r#"{"repo":"x"}"#),
            ("dry_run", r#"{"enabled":true}"#),
            ("send_message", r#"{"to":"bob","text":"hi"}"#),
            ("kv_get", r#"{"key":"theme"}"#),
            ("translate", r#"{"source":"en"}"#),
        ] {
            assert!(!HITLApprover::is_high_risk_call(tool, args),
                "{tool} + {args} must stay allowed (compound name / field, not a verb)");
        }
        /* The residual compound danger is DECLARED, not silently claimed fixed: `delete_database`
         * is not caught by any list, and that is the measured proof that the NAME axis is
         * undecidable (criterion ⑤) — it needs a capability declaration, not more words. */
        assert!(!HITLApprover::is_high_risk_call("delete_database", r#"{"target":"prod"}"#),
            "residual: compound danger is undeclared-undecidable on the name axis");
    }

    #[test]
    fn dangerous_verbs_by_name_are_restored_without_reviving_the_false_positives() {
        /* §220 ③ measured this regression: after §216 these were all allowed. A tool that IS the
         * verb is dangerous regardless of its arguments. */
        for (tool, args) in [
            ("rm", r#"{"path":"/data"}"#),
            ("curl", r#"{"url":"http://x"}"#),
            ("ssh", r#"{"host":"h"}"#),
            ("wget", r#"{"url":"http://x"}"#),
            ("dd", r#"{"if":"/dev/zero"}"#),
            ("sudo", r#"{"argv":["ls"]}"#),
            ("mkfs", r#"{"dev":"/dev/sdb"}"#),
            ("chmod", r#"{"mode":"777"}"#),
        ] {
            assert!(HITLApprover::is_high_risk_call(tool, args),
                "{tool} IS the dangerous verb ⇒ must be high-risk by name");
        }
        /* AND the names that were measured false positives must stay allowed (compound / generic). */
        for tool in ["code_review", "dry_run", "send_message", "run_query", "process_document",
                     "shell_completion", "service_status", "mount_info", "translate", "render"] {
            assert!(!HITLApprover::is_high_risk_call(tool, r#"{"x":1}"#),
                "{tool} is a compound name, not a dangerous verb");
        }
    }

    #[test]
    fn compound_names_and_generic_fields_are_not_capabilities() {
        /* §216 channels A and B — every one of these was a measured false positive. */
        for (tool, args) in [
            ("code_review", r#"{"repo":"x"}"#),
            ("process_document", r#"{"doc_id":"d1"}"#),
            ("invoke_api", r#"{"endpoint":"/v1/ping"}"#),
            ("script_lint", r#"{"file":"a.py"}"#),
            ("shell_completion", r#"{"line":"ls"}"#),
            ("run_query", r#"{"sql":"select 1"}"#),
            ("service_status", r#"{"name":"nginx"}"#),
            ("mount_info", r#"{"dev":"/dev/sda"}"#),
            /* ★ the inverted case: the one tool that does NOT run */
            ("dry_run", r#"{"enabled":true}"#),
            /* ★ "do not run" read as "run": Bool must not count as carrying a command */
            ("background_job", r#"{"run":false}"#),
            ("build", r#"{"run_id":42}"#),
            /* generic field names from ordinary schemas */
            ("translate", r#"{"source":"en"}"#),
            ("llm_complete", r#"{"input":"write a haiku"}"#),
            ("embeddings", r#"{"input":["a","b"]}"#),
            ("form_submit", r#"{"action":"submit"}"#),
            ("render", r#"{"payload":{"a":1}}"#),
            ("analyze", r#"{"expression":"a+b"}"#),
        ] {
            assert!(!HITLApprover::is_high_risk_call(tool, args),
                "{tool} + {args}: a compound name or a generic field is not a capability");
        }
    }

    #[test]
    fn the_refusal_reason_must_not_name_its_trigger() {
        /* §216 criterion ⑤ (work factor applied to the FEEDBACK channel): a refusal that says
         * "because you used the `code` field" teaches the adversary which name to change next. The
         * reason must therefore be a capability statement, not a citation of the input. */
        let reason = HITLApprover::capability_undeclared_reason();
        assert!(reason.contains("capability-undeclared"), "{reason}");
        for leak in ["code", "script", "cmd", "run_command", "field", "args"] {
            assert!(!reason.to_lowercase().contains(leak), "reason leaks {leak}: {reason}");
        }
    }

    #[test]
    fn data_is_not_a_command_and_a_command_is_not_decided_by_its_text() {
        /* §215 A: these are DATA. Blocking them was the 91.7% false-positive rate. */
        let harmless = [
            ("read_file", r#"{"path":"/home/user/.ssh/id_rsa.pub"}"#),
            ("search", r#"{"query":"how to remove a stain"}"#),
            ("translate", r#"{"text":"please delete the draft","lang":"en"}"#),
            ("http_get", r#"{"url":"https://api.example.com/status"}"#),
            ("send_message", r#"{"to":"bob","text":"send my regards"}"#),
            ("kv_get", r#"{"key":"theme","value":"dark"}"#),
            ("db_query", r#"{"sql":"SELECT * FROM users WHERE token = ?"}"#),
            ("list_dir", r#"{"path":"/var/www"}"#),
            ("summarize", r#"{"doc":"Chapter 12: the key to the kingdom"}"#),
            ("render", r#"{"template":"<a href='https://x'>post</a>"}"#),
            ("config_read", r#"{"key":"timeout"}"#),
        ];
        for (tool, args) in harmless {
            assert!(!HITLApprover::is_high_risk_call(tool, args),
                "{tool} + {args}: data must not be read as a command");
        }
        /* §215 B: `key` is the commonest KV field name on earth, and it was in CREDENTIAL with a
         * substring match — so this is the shape that made the list unusable. */
        for probe in [r#"{"monkey":"x"}"#, r#"{"keyword":"x"}"#, r#"{"donkey":"x"}"#] {
            assert!(!HITLApprover::is_high_risk_call("kv_get", probe), "{probe} is not a credential");
        }

        /* §215 C: the ELABORATE dangerous forms. They contain no list token — and they must still be
         * refused, because the refusal cites the SHAPE (a command-bearing field), not the words. */
        let elaborate = [
            ("code_runner", r#"{"code":"import base64,os;os.system(base64.b64decode('cm0gLXJmIC8=').decode())"}"#),
            ("executor", r#"{"code":"__import__('os').system(chr(114)+chr(109)+' -rf /')"}"#),
            ("shell", r#"{"script":"ls -la"}"#),
            ("run_command", r#"{"cmd":"rm -rf /data"}"#),
        ];
        for (tool, args) in elaborate {
            assert!(HITLApprover::is_high_risk_call(tool, args),
                "{tool} + {args}: a command-bearing field is undecidable ⇒ refuse, text-independent");
        }
        /* And undecidable input is not permission (B7). */
        assert!(HITLApprover::is_high_risk_call("mystery", "not json at all"));
    }

    #[test]
    fn capability_axis_is_high_risk_regardless_of_the_name() {
        /* §212 measured: `bash`/`python3`/`sudo`/`docker` were NOT high-risk while `rm`/`curl` were,
         * so the blacklist blocked names and released capability (0.0% on that axis). */
        for t in ["bash", "sh", "python3", "node", "perl", "ruby", "eval", "exec",
                  "sudo", "docker", "kubectl", "terraform", "pip", "npm", "cargo", "make", "crontab"] {
            assert!(HITLApprover::is_high_risk(t), "{t} runs anything ⇒ must be high-risk");
        }
        /* And the axis must not eat the genuinely low-risk case, or the classifier becomes noise. */
        for t in ["ls", "cat", "read_file", "grep", "head", "wc"] {
            assert!(!HITLApprover::is_high_risk(t), "{t} is a read ⇒ must not be high-risk");
        }
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
