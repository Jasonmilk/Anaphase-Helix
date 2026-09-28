//! HITL 人在回路审批通道（P10b T3，DNA 原则 4）。
//!
//! 三层闸门之一（执行闸）：工具审计（入库门）→ **HITL（执行闸）** → Tuck（边缘物理闸）。
//! HITL 管"这次动作能不能执行"：高风险动作（写操作/网络请求/凭证使用）必须经人类确认，
//! 未经确认被物理拦截。低风险动作零额外延迟（直接放行）。
//!
//! 默认 **fail-closed**：无人类确认通道时，高风险动作拦截（HITL 物理语义，安全优先）。

use std::sync::Arc;

/// 人类确认回调：`(command, args) -> Ok(true)=确认放行 / Ok(false)=拒绝 / Err=无通道`
/// THE SINGLE WORD TABLE (ADR-0048 §231). Every action word this crate knows, WITH THE ROLES IT
/// SERVES — one host for the words, and the differences between classifiers become DECLARED tags
/// instead of two hand-written lists that drift (measured: two same-named EXECUTION constants of
/// 83 and 77 words, differing in ten places each, both hidden inside function bodies).
///
/// Roles
///   A  executor NAME, matched with ALL tokens   (`is_executor_name`: a name whose every word runs)
///   B  COMMAND string, matched with ANY token   (`is_high_risk`: a command containing a risky word)
///   S  the CREDENTIAL subset used by SUBSTRING matching in the command role
///   CD dangerous BARE name, and dangerous when ALL tokens match   (`is_dangerous_name`)
///   C  dangerous BARE name only (generic words that must not fire on a compound)
const ACTION_WORDS: &[(&str, &str)] = &[
    ("ansible", "AB"),
    ("api_key", "BS"),
    ("apikey", "C"),
    ("apt", "AB"),
    ("apt-get", "B"),
    ("at", "B"),
    ("auth", "C"),
    ("awk", "AB"),
    ("bash", "AB"),
    ("bearer", "BSC"),
    ("brew", "AB"),
    ("bun", "AB"),
    ("bundle", "AB"),
    ("cargo", "AB"),
    ("chgrp", "ABCD"),
    ("chmod", "ABCD"),
    ("chown", "ABCD"),
    ("cmake", "AB"),
    ("cmd", "AB"),
    ("code", "A"),
    ("command", "A"),
    ("computeruse", "A"),
    ("cookie", "BSC"),
    ("copy", "C"),
    ("cp", "BC"),
    ("credential", "BSC"),
    ("credentials", "C"),
    ("crontab", "ABCD"),
    ("csh", "AB"),
    ("curl", "BCD"),
    ("dash", "AB"),
    ("dd", "BCD"),
    ("delete", "BC"),
    ("deno", "AB"),
    ("dispatch", "A"),
    ("dnf", "AB"),
    ("doas", "ABCD"),
    ("docker", "AB"),
    ("env", "B"),
    ("eval", "AB"),
    ("exec", "AB"),
    ("execute", "A"),
    ("executor", "A"),
    ("fdisk", "CD"),
    ("fetch", "BC"),
    ("fish", "AB"),
    ("format", "CD"),
    ("ftp", "CD"),
    ("gem", "AB"),
    ("go", "B"),
    ("gradle", "AB"),
    ("halt", "CD"),
    ("helm", "AB"),
    ("http", "BC"),
    ("https", "BC"),
    ("interpreter", "A"),
    ("invoke", "A"),
    ("julia", "AB"),
    ("key", "BSC"),
    ("kill", "CD"),
    ("killall", "CD"),
    ("ksh", "AB"),
    ("kubectl", "AB"),
    ("launchctl", "ABCD"),
    ("lua", "AB"),
    ("make", "AB"),
    ("maven", "B"),
    ("mkdir", "BC"),
    ("mkfs", "CD"),
    ("mount", "ABCD"),
    ("move", "C"),
    ("mv", "BC"),
    ("mvn", "AB"),
    ("nc", "BCD"),
    ("ncat", "BCD"),
    ("netcat", "CD"),
    ("node", "AB"),
    ("nodejs", "AB"),
    ("npm", "AB"),
    ("npx", "AB"),
    ("osremove", "B"),
    ("pacman", "AB"),
    ("passwd", "BC"),
    ("password", "BSC"),
    ("perl", "AB"),
    ("php", "AB"),
    ("pip", "AB"),
    ("pip3", "AB"),
    ("pkexec", "ABCD"),
    ("pkill", "CD"),
    ("pnpm", "AB"),
    ("podman", "AB"),
    ("post", "BC"),
    ("powershell", "AB"),
    ("process", "A"),
    ("pwsh", "AB"),
    ("py", "AB"),
    ("python", "AB"),
    ("python2", "AB"),
    ("python3", "AB"),
    ("r", "B"),
    ("reboot", "CD"),
    ("remove", "BC"),
    ("request", "C"),
    ("rm", "BCD"),
    ("rmdir", "BC"),
    ("ruby", "AB"),
    ("run", "A"),
    ("runcommand", "A"),
    ("sandbox", "A"),
    ("scp", "BCD"),
    ("script", "A"),
    ("secret", "BSC"),
    ("sed", "AB"),
    ("send", "BC"),
    ("service", "AB"),
    ("sftp", "BCD"),
    ("sh", "AB"),
    ("shell", "A"),
    ("shred", "BCD"),
    ("shutdown", "CD"),
    ("shutil", "B"),
    ("source", "B"),
    ("spawn", "A"),
    ("ssh", "BCD"),
    ("su", "ABCD"),
    ("sudo", "ABCD"),
    ("systemctl", "ABCD"),
    ("tcsh", "AB"),
    ("telnet", "CD"),
    ("terminal", "A"),
    ("terraform", "AB"),
    ("token", "BSC"),
    ("touch", "BC"),
    ("truncate", "BCD"),
    ("umount", "ABCD"),
    ("unlink", "BCD"),
    ("unlinkat", "B"),
    ("upload", "C"),
    ("useradd", "B"),
    ("usermod", "B"),
    ("vagrant", "AB"),
    ("wget", "BCD"),
    ("write", "BC"),
    ("xargs", "AB"),
    ("yarn", "AB"),
    ("yum", "AB"),
    ("zsh", "AB"),
];

/// The DERIVED view a classifier reads: every word tagged with at least one of `roles`.
/// It is a projection of the one table — never a second copy.
fn role_words(roles: &str) -> Vec<&'static str> {
    ACTION_WORDS
        .iter()
        .filter(|(_, r)| r.chars().any(|c| roles.contains(c)))
        .map(|(w, _)| *w)
        .collect()
}

pub type ApproveFn = Arc<dyn Fn(&str, &[String]) -> ApprovalOutcome + Send + Sync>;

/// 把**旧布尔合同**装进闭集（ADR-0048 §226）:`Ok(true) ⇒ AllowedOnce` · `Ok(false) ⇒ Rejected` ·
/// `Err(_) ⇒ Unavailable`。旧批准者不必改写,而**新批准者可以表达 `Cancelled`**
/// —— 这是"加宽管子",不是"催工人接线"。
pub fn from_bool(
    f: Arc<dyn Fn(&str, &[String]) -> Result<bool, String> + Send + Sync>,
) -> ApproveFn {
    Arc::new(move |c, a| match f(c, a) {
        Ok(true) => ApprovalOutcome::AllowedOnce,
        Ok(false) => ApprovalOutcome::Rejected,
        Err(_) => ApprovalOutcome::Unavailable,
    })
}

pub struct HITLApprover {
    approver: ApproveFn,
}

impl Default for HITLApprover {
    fn default() -> Self {
        // fail-closed：无确认通道时，高风险动作拦截（Err = 通道缺失）
        Self {
            approver: Arc::new(|_cmd, _args| {
                ApprovalOutcome::Unavailable
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
        /* THE OTHER GATE'S HALF (ADR-0048 §222). Measured: the ENGINE gate's list (`is_high_risk`)
         * and this pipeline gate's list shared only 79 words (Jaccard 0.577) — 30 words were blocked
         * by the engine and invisible here, and this module's own header says high-risk means
         * "write / network / credential". The pipeline gate scored **0/20** on bare names for those
         * three classes, so `delete_database {"target":"prod"}` and `read_secret {"name":"api_key"}`
         * reached Tentacle without confirmation on this path.
         * These are BARE-name words: a tool called `key` is a credential reader, while a FIELD called
         * `key` is not scanned by this predicate at all (that was §216's false-positive channel). */
        let lower = tool.to_lowercase();
        let bare = lower.replace(['_', '-'], "");
        if role_words("CD").contains(&bare.as_str())
            || role_words("C").contains(&bare.as_str())
            || role_words("C").contains(&bare.as_str())
            || role_words("C").contains(&bare.as_str())
            || role_words("C").contains(&bare.as_str())
            || role_words("C").contains(&bare.as_str())
        {
            return true;
        }
        let toks: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        !toks.is_empty()
            && toks.iter().all(|t| {
                role_words("D").contains(t)
            })
    }

    /// Is this tool itself a program runner? Matched on its own tokens against the EXECUTION
    /// capability list — the one place where the name really is the capability.
    pub fn is_executor_name(tool: &str) -> bool {
        let lower = tool.to_lowercase();
        let bare = lower.replace(['_', '-'], "");
        if role_words("A").contains(&bare.as_str()) {
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
        !toks.is_empty() && toks.iter().all(|t| role_words("A").contains(t))
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
        /* ABSENT IS NOT MALFORMED (ADR-0048 §225, measured the hard way): the first version of this
         * rule treated an EMPTY args string as undecidable and refused — which made every call with
         * no arguments high-risk, and two existing tests caught it (a low-risk tool was suddenly
         * consulted). "No arguments" is a fact we understand; "unparseable arguments" is the one we
         * cannot decide. B7 applies to the second. */
        if args_json.trim().is_empty() {
            return false;
        }
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
        let cmd = command.to_lowercase();
        let tokens: Vec<String> = cmd
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        for t in &tokens {
            if role_words("B").contains(&t.as_str())
                || role_words("B").contains(&t.as_str())
                || role_words("S").contains(&t.as_str())
                || role_words("B").contains(&t.as_str())
            {
                return true;
            }
        }
        // 凭证子串匹配（如 my_api_key / send_credentials）
        let compact = cmd.replace(['_', '-'], "");
        role_words("S").iter().any(|k| compact.contains(*k))
    }

    /// HITL 执行闸：低风险 → 直接放行；高风险 → 请求人类确认
    pub fn check_approval(&self, command: &str, args: &[String]) -> ApprovalOutcome {
        /* 不变量(§226 ⑤):`args` 恰好携带一个已序列化的 JSON 值;多余者会被静默忽略。 */
        debug_assert!(args.len() <= 1, "check_approval expects one serialized args value");
        /* THE UNION, NOT THE REPLACEMENT (ADR-0048 §225, corrected). §225 proposed switching this
         * to `is_high_risk_call(tool, args_json)`; measured, that would LOSE the engine's recall,
         * because `is_dangerous_name("rm -rf /")` is `false` (tokens {rm, rf}: `rf` is not in any
         * list, so the all-tokens rule declines) while the old token-OR classifier catches it.
         * The engine keeps its command judgement AND gains the args-aware one. */
        let args_json = args.first().cloned().unwrap_or_default();
        if !Self::is_high_risk(command) && !Self::is_high_risk_call(command, &args_json) {
            return ApprovalOutcome::AllowedOnce;
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
    fn command_variants_of_a_destructive_verb_are_all_intercepted() {
        /* The owner asked: `rm -rf`, `rm  -rf`, ` rm  -rf` … are they all intercepted?
         * Answered by RUNNING it, not by memory. The tokenizer splits on every non-alphanumeric and
         * drops empties, and matching is case-insensitive — so spacing, leading/trailing whitespace,
         * path prefixes, flags and case are all irrelevant. */
        for cmd in [
            "rm -rf /",
            "rm  -rf  /",
            "   rm -rf /   ",
            "RM -RF /",
            "Rm -Rf /data",
            "/bin/rm -rf /",
            "./rm -rf /",
            "sudo rm -rf /",
            "rm\t-rf /",
            "rm\n-rf /",
            "sh -c 'rm -rf /'",
            "rmdir /data",
            "truncate -s 0 /etc/passwd",
        ] {
            assert!(HITLApprover::is_high_risk(cmd), "is_high_risk must catch: {cmd:?}");
        }
        /* AND the honest other side: the tokenizer can be defeated by splitting the word itself.
         * These are MISSES, and they are asserted as misses so they are recorded rather than
         * assumed away (criterion §222.4: a known gap that is asserted beats one that is not). */
        /* `r''m` / `r\\m` are CAUGHT — measured, and only incidentally: the tokenizer splits them
         * into {r, m}, and `r` is in the list as the R interpreter. That is luck, not design, and it
         * is worth recording as luck. The genuine misses are words that are NOT in any list: */
        for caught_by_luck in ["r''m -rf /", "r\\m -rf /", "r m -rf /"] {
            assert!(HITLApprover::is_high_risk(caught_by_luck),
                "{caught_by_luck:?} is caught only because `r` is listed (the R interpreter)");
        }
        for evasion in ["rmm -rf /", "rnm -rf /", "del -rf /", "purge /data", "$(echo cm0=)"] {
            assert!(!HITLApprover::is_high_risk(evasion),
                "KNOWN MISS (asserted, not assumed): {evasion:?} defeats a WORD list");
        }
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
    fn one_word_table_with_declared_roles() {
        /* §231's criteria, all four in one place. */
        assert!(ACTION_WORDS.len() >= 100, "the table is the single host: {} words", ACTION_WORDS.len());
        /* ① one literal: every role view is a projection of THIS table (nothing else defines words). */
        for r in ["A", "B", "S", "C", "D"] {
            let view = role_words(r);
            assert!(!view.is_empty(), "role {r} has a declared, non-empty view");
            for w in &view {
                assert!(ACTION_WORDS.iter().any(|(t, _)| t == w), "{w} in role {r} comes from the table");
            }
        }
        /* ② no word is listed twice (a duplicate would be a second copy hiding in the table). */
        let mut seen = std::collections::HashSet::new();
        for (w, _) in ACTION_WORDS {
            assert!(seen.insert(*w), "duplicate word in the table: {w}");
        }
        /* ③ the A-role difference is DECLARED: `run` may name an executor without making a command
         *    risky — the tags say so, rather than two lists disagreeing by accident. */
        assert!(role_words("A").contains(&"run") && !role_words("B").contains(&"run"),
            "`run` is tagged for the executor-name role only, and that is written down");
        /* ④ THE FALSE-POSITIVE GUARD: unifying must not widen the COMMAND role (that was §231's trap). */
        assert!(!HITLApprover::is_high_risk("echo run tests"), "ordinary prose must not become high-risk");
        assert!(!HITLApprover::is_high_risk("ls -la"), "a read command stays low-risk");
        assert!(!HITLApprover::is_high_risk("git log --oneline"), "and so does a normal git command");
        /* …while recall is kept: the destructive verb still fires. */
        assert!(HITLApprover::is_high_risk("rm -rf /data"));
        assert!(HITLApprover::is_high_risk("curl https://example.com"));
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
        assert_eq!(h.check_approval("echo", &[]), ApprovalOutcome::AllowedOnce);
    }

    #[test]
    fn high_risk_fail_closed_without_channel() {
        // 高风险 + 无确认通道 → fail-closed 拦截（Err）
        let h = HITLApprover::default();
        assert_eq!(h.check_approval("rm -rf /data", &[]), ApprovalOutcome::Unavailable);
    }

    #[test]
    fn high_risk_approve_deny() {
        let approve = HITLApprover::new(Arc::new(|_c, _a| ApprovalOutcome::AllowedOnce));
        assert_eq!(approve.check_approval("curl http://x", &[]), ApprovalOutcome::AllowedOnce);

        let deny = HITLApprover::new(Arc::new(|_c, _a| ApprovalOutcome::Rejected));
        assert_eq!(deny.check_approval("curl http://x", &[]), ApprovalOutcome::Rejected);
    }
}
