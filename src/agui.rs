//! AG-UI 1.0 **additive 投影**（C-083 / 队列坐标 L2）。
//!
//! 边界（刻意写死，别扩）：
//! - **只映射真实骨架里有的**：`turn/start` → `RUN_STARTED`，`assistant/reply` →
//!   `TEXT_MESSAGE_START/CONTENT/END`，`turn/end` → `RUN_FINISHED`。没有工具段就**不造** `TOOL_CALL_*`。
//! - **metadata 只挂 `verdict`**：`criteria`/`trace` 本轮不落（队尾 revisit；不派去标准事件或 CUSTOM）。
//! - **纯函数、无状态、无写入**：删掉整个模块，TUI/Web 与账本不受影响（additive）。
//! - `threadId` / `runId` 取自已有的 `period_id` / `job_id`；不解析其结构（协议要求 opaque）。
//!
//! ★ 章 B 对齐（2026-11-20，按官方副本 sha256_16=4b5c93226838a0e7）：
//!   - 只保留**官方定义里声明过的键**：`TEXT_MESSAGE_*` 去掉 threadId/runId（官方 required 只有 messageId），
//!     `TOOL_CALL_*` 同理；`TOOL_CALL_RESULT` **补 messageId**（官方 required）；
//!     `RUN_FINISHED` 去掉非官方 `metadata`（verdict 暂无诚实的官方落点 ⇒ 记「未覆盖」，不猜进 result/outcome）。
//!   - 官方有而我们**不映射**的类型列在 `UNCOVERED`，显式「未覆盖」而不是假装没有位置。
use serde_json::{json, Value};

/// 官方 AG-UI 1.0 里**存在、但我们不投影**的类型（显式「未覆盖」，不是"协议没位置"）。
pub const UNCOVERED: &[&str] = &[
    "STEP_STARTED", "STEP_FINISHED", "RUN_ERROR",
    "TOOL_CALL_ARGS", "TOOL_CALL_CHUNK",
    "REASONING_START", "REASONING_END", "REASONING_MESSAGE_START",
    "REASONING_MESSAGE_CONTENT", "REASONING_MESSAGE_END", "REASONING_MESSAGE_CHUNK",
    "REASONING_ENCRYPTED_VALUE",
    "STATE_SNAPSHOT", "STATE_DELTA", "MESSAGES_SNAPSHOT",
    "ACTIVITY_SNAPSHOT", "ACTIVITY_DELTA",
    "TEXT_MESSAGE_CHUNK", "RAW", "CUSTOM", "META_EVENT",
];

/// 一条 session event → 0..n 条 AG-UI 1.0 事件。认不出的类型返回**空**（不静默造事件）。
pub fn project(ev: &Value) -> Vec<Value> {
    let ty = ev.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let thread = ev.get("period_id").and_then(|v| v.as_str()).unwrap_or("");
    let run = ev.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
    let data = ev.get("data").cloned().unwrap_or_else(|| json!({}));
    match ty {
        "turn/start" => vec![json!({
            "type": "RUN_STARTED", "threadId": thread, "runId": run
        })],
        "assistant/reply" => {
            let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("");
            vec![
                json!({"type":"TEXT_MESSAGE_START","messageId":run,"role":"assistant"}),
                json!({"type":"TEXT_MESSAGE_CONTENT","messageId":run,"delta":text}),
                json!({"type":"TEXT_MESSAGE_END","messageId":run}),
            ]
        }
        "turn/end" => {
            // 章 B：官方 RUN_FINISHED 的键是 type/threadId/runId/(result|outcome|usage)。
            // 我们手上只有 verdict（Met/Unmet/null），它**不是** return value，也不是
            // success/interrupt/cancelled 三态之一 ⇒ **不猜进 result/outcome**，记「未覆盖」。
            let _verdict = data.get("verdict").cloned().unwrap_or(Value::Null);
            vec![json!({
                "type": "RUN_FINISHED", "threadId": thread, "runId": run
            })]
        }
        // ★ B1（2026-11-19 真跑撞到）：`tool/call` / `tool/result` 是**真实存在**的事件类型
        // （本仓已有带 tool/call 的真跑 tape），而投影此前直接落到 `_ => vec![]` ⇒ 工具段在
        // AG-UI 出口里**整段消失**。此前的延后条件（"等真走工具段的周期再长"）现已满足。
        // 只映射到 AG-UI 1.0 已有的标准类型，不新造语义、不改协议。
        "tool/call" => {
            let idx = data.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            let name = data.get("tool").and_then(|v| v.as_str()).unwrap_or("");
            let id = format!("{run}#{idx}");
            vec![json!({
                "type": "TOOL_CALL_START",
                "toolCallId": id, "toolCallName": name
            })]
        }
        "tool/result" => {
            let idx = data.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            let id = format!("{run}#{idx}");
            let content = data.get("outcome").cloned().unwrap_or(Value::Null);
            let ok = data.get("ok").cloned().unwrap_or(Value::Bool(false));
            // 官方 TOOL_CALL_RESULT 必填 [type,messageId,toolCallId,content] ⇒ 补 messageId（本条结果成为的消息）
            let msg_id = format!("{run}#tool{idx}");
            let _ok = ok;
            vec![
                json!({"type":"TOOL_CALL_END","toolCallId":id}),
                json!({"type":"TOOL_CALL_RESULT","messageId":msg_id,
                       "toolCallId":id,"content":content}),
            ]
        }
        _ => vec![],
    }
}

/// 一次周期的事件序列 → AG-UI 序列。顺序保持不变（协议按到达序解释）。
pub fn project_all(events: &[Value]) -> Vec<Value> {
    events.iter().flat_map(|e| project(e)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ty: &str, data: Value) -> Value {
        json!({"type": ty, "period_id": "p1", "job_id": "j1", "data": data})
    }

    /// 形 1：真实 7 行骨架 → 5 条 AG-UI 事件，且首尾是 RUN_STARTED / RUN_FINISHED。
    #[test]
    fn the_real_skeleton_maps_to_five_standard_events() {
        let rows = vec![
            ev("turn/start", json!({})),
            ev("user/message", json!({"text": "hi"})),
            ev("context/inject", json!({})),
            ev("assistant/attempt", json!({})),
            ev("assistant/reply", json!({"text": "pong"})),
            ev("turn/end", json!({"verdict": "MET"})),
            ev("check/status", json!({"phase": "hitl"})),
        ];
        let out = project_all(&rows);
        let types: Vec<&str> = out.iter().filter_map(|e| e["type"].as_str()).collect();
        assert_eq!(types, vec!["RUN_STARTED", "TEXT_MESSAGE_START", "TEXT_MESSAGE_CONTENT", "TEXT_MESSAGE_END", "RUN_FINISHED"]);
        // 章 B（2026-11-20）：RUN_FINISHED 只保留官方键（type/threadId/runId）；
        // verdict 暂无诚实的官方落点 ⇒ **不挂 metadata、也不猜进 result/outcome**（记「未覆盖」）。
        assert_eq!(out[4]["type"], "RUN_FINISHED");
        assert!(out[4].get("metadata").is_none(), "官方 RUN_FINISHED 没有 metadata 键");
        assert!(out[4].get("result").is_none() && out[4].get("outcome").is_none(), "不猜 verdict 的落点");
    }

    /// 形 2：`NoToolNeeded` 周期**不得**产出任何 `TOOL_CALL_*`（不预画骨架）。
    #[test]
    fn a_no_tool_cycle_never_fabricates_tool_events() {
        let rows = vec![
            ev("turn/start", json!({})),
            ev("assistant/reply", json!({"text": "1+1=2"})),
            ev("turn/end", json!({"verdict": Value::Null})),
        ];
        let out = project_all(&rows);
        assert!(!out.iter().any(|e| e["type"].as_str().unwrap_or("").starts_with("TOOL_CALL")));
    }

    /// 形 3：认不出的类型 ⇒ 空投影（读到空要具名，不许造事件）。
    #[test]
    fn an_unknown_event_projects_to_nothing() {
        assert!(project(&ev("made/up", json!({}))).is_empty());
    }

    /// ★ B1（2026-11-19）：真跑里存在 `tool/call`/`tool/result` ⇒ 必须投影成标准工具事件；
    /// 反向（原本的守卫）：没有工具段 ⇒ 不得出现 TOOL_CALL_*（防伪造）。
    #[test]
    fn tool_events_are_projected_and_not_invented() {
        let with_tool = vec![
            ev("turn/start", json!({})),
            ev("tool/call", json!({"tool":"calc","index":0,"expect":"ok"})),
            ev("tool/result", json!({"tool":"calc","index":0,"ok":true,"outcome":"{\"ok\":true}"})),
            ev("turn/end", json!({})),
        ];
        let out = project_all(&with_tool);
        let types: Vec<&str> = out.iter().filter_map(|e| e["type"].as_str()).collect();
        assert!(types.contains(&"TOOL_CALL_START"), "{types:?}");
        assert!(types.contains(&"TOOL_CALL_END"), "{types:?}");
        assert!(types.contains(&"TOOL_CALL_RESULT"), "{types:?}");

        let without = vec![ev("turn/start", json!({})), ev("turn/end", json!({}))];
        let out2 = project_all(&without);
        assert!(!out2.iter().any(|e| e["type"].as_str().unwrap_or("").starts_with("TOOL_CALL")));
    }

    /// 形 4（章 B 改写）：`turn/end` 里即便带 verdict/criteria/trace，投影也**只**输出官方键。
    #[test]
    fn run_finished_carries_only_official_keys() {
        let out = project(&ev("turn/end", json!({"verdict": "UNMET", "criteria": [1], "trace": "t"})));
        let mut keys: Vec<&str> = out[0].as_object().expect("object").keys().map(|k| k.as_str()).collect();
        keys.sort();
        assert_eq!(keys, vec!["runId", "threadId", "type"]);
    }

    /// 形 5（章 B）：官方必填键 —— TEXT_MESSAGE_* 只带 messageId（+START 的 role），
    /// TOOL_CALL_RESULT 必须带 messageId。用官方副本的 required 当尺。
    #[test]
    fn text_and_tool_result_carry_official_required_keys() {
        let t = project(&ev("assistant/reply", json!({"text": "hi"})));
        for e in &t {
            assert!(e.get("messageId").is_some(), "TEXT_MESSAGE_* 缺 messageId: {e}");
            assert!(e.get("threadId").is_none() && e.get("runId").is_none(), "官方 TEXT_MESSAGE_* 没有 threadId/runId: {e}");
        }
        let r = project(&ev("tool/result", json!({"tool":"calc","index":0,"ok":true,"outcome":"ok"})));
        let res = r.iter().find(|e| e["type"] == "TOOL_CALL_RESULT").expect("result");
        assert!(res.get("messageId").is_some(), "官方 TOOL_CALL_RESULT 必填 messageId: {res}");
        assert!(res.get("metadata").is_none(), "官方 TOOL_CALL_RESULT 没有 metadata 键");
    }
}
