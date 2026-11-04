//! AG-UI 1.0 **additive 投影**（C-083 / 队列坐标 L2）。
//!
//! 边界（刻意写死，别扩）：
//! - **只映射真实骨架里有的**：`turn/start` → `RUN_STARTED`，`assistant/reply` →
//!   `TEXT_MESSAGE_START/CONTENT/END`，`turn/end` → `RUN_FINISHED`。没有工具段就**不造** `TOOL_CALL_*`。
//! - **metadata 只挂 `verdict`**：`criteria`/`trace` 本轮不落（队尾 revisit；不派去标准事件或 CUSTOM）。
//! - **纯函数、无状态、无写入**：删掉整个模块，TUI/Web 与账本不受影响（additive）。
//! - `threadId` / `runId` 取自已有的 `period_id` / `job_id`；不解析其结构（协议要求 opaque）。
use serde_json::{json, Value};

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
                json!({"type":"TEXT_MESSAGE_START","threadId":thread,"runId":run,"messageId":run,"role":"assistant"}),
                json!({"type":"TEXT_MESSAGE_CONTENT","threadId":thread,"runId":run,"messageId":run,"delta":text}),
                json!({"type":"TEXT_MESSAGE_END","threadId":thread,"runId":run,"messageId":run}),
            ]
        }
        "turn/end" => {
            // metadata 只挂 verdict：它不是"标准事件的替代品"，而是这条事件自己的注解。
            let verdict = data.get("verdict").cloned().unwrap_or(Value::Null);
            vec![json!({
                "type": "RUN_FINISHED", "threadId": thread, "runId": run,
                "metadata": {"verdict": verdict}
            })]
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
        assert_eq!(out[4]["metadata"]["verdict"], "MET");
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

    /// 形 4：metadata 只挂 verdict —— criteria/trace 不许出现在这里。
    #[test]
    fn metadata_carries_verdict_and_nothing_else() {
        let out = project(&ev("turn/end", json!({"verdict": "UNMET", "criteria": [1], "trace": "t"})));
        let md = out[0]["metadata"].as_object().expect("metadata object").clone();
        assert_eq!(md.len(), 1);
        assert!(md.contains_key("verdict"));
    }
}
