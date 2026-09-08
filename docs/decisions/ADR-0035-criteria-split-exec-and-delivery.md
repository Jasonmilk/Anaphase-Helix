# ADR-0035: 判据契约修复——exec_ok 与 answer.delivered 分层

- **状态**: Accepted
- **日期**: 2026-09-09
- **决策范围**: Anaphase（criteria / Expect::Ok 映射）
- **关联**: Cellrix ADR-0015（Engram 判据审查）、ADR-0029（判据不读思考）
- **取代**: 原 `exec_ok(ok_flag, echoed)` 单判据

## 1. 背景与问题

真实联调（`run-35718410f20c6cda`）暴露假阴性：calc 工具成功执行
`{"ok":true,"result":"40353607"}`，答案已通过 SSE 回传用户，但判据
`exec_ok` 判 **Unmet**。

根因（物理事实）：

```
exec_ok 检查 ok_flag && echoed
echoed  = data.data.params 非空   ← 绑定 mcp_proxy.js 的 {"ok":true,"data":{"params":...}} 包装
calc 返回 {"ok":true,"result":"40353607"}   ← Tentacle 平面契约，无 data.params
→ echoed 恒 false → 假阴性
```

判据把"工具执行成功"与"答案交付"耦合，且用错误的代理指标（mcp 包装）
检测交付——正是用户审查 Engram 时预言的"判据名实不符"。

## 2. 决策

拆成两条，各归各位（判据层能看到的最强物理事实）：

| check | 层 | 检测点 | 判定 |
|---|---|---|---|
| `exec_ok` | 工具层 | `ok == true` | 工具执行成功 |
| `answer.delivered` | 交付层（工具边界） | 业务结果字段非空（`result`/`series`/`numerator`/`denominator`/`trend_a`/`trend_b`）或 mcp `data.params` 回传 | 产出了业务结果 |

两种契约都通过：Tentacle 平面返回 + mcp_proxy 包装。

## 3. 边界

- 判据层测不了 SSE 回传（那是 /v1/chat handler 的运输事实）——
  `answer.delivered` 以"工具产出业务结果"为判据层可审计的最强事实，
  不越界猜测。
- `ok=false` 或"成功但无业务结果"仍 fail-closed。
- 测试：`ok_mapping_flat_contract_passes` / `ok_mapping_fails_closed_on_missing_result`。

## 4. 验证

真实 tokens 联调（`run-30fabc23294f31e6`）：calc 8**6 → `exec_ok` pass +
`answer.delivered` pass → **Met**（修复前恒 Unmet）。`turn/end success=true
≡ verdict=Met`（ADR-0015 铁律保持）。
