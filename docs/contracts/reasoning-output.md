# Reasoning 输出协议（candidate E，ADR-0005）

> **用途**：run_cycle Reasoning 状态与确定性 pipeline 之间的结构化契约。
> **替代**：legacy `contains("tool_call")` 字符串匹配（已删除）。
> **解析入口**：`contract::parse_reasoning_output`（唯一消费点）。

## 协议形状

Reasoning 输出（`ReasoningAdapter::reason()` 返回的 content 字符串）必须是 JSON，两种形态：

### 形态 1：包装对象（推荐，支持 impasse 标志）

```json
{
  "calls": [
    { "tool": "numbers", "args": {}, "expect": "numbers" },
    { "tool": "rate",    "args": {}, "expect": "rate" }
  ],
  "impasse": false
}
```

### 形态 2：裸 calls 数组（兼容 M1 形态）

```json
[ { "tool": "numbers", "args": {}, "expect": "numbers" } ]
```

## 字段语义

| 字段 | 类型 | 必填 | 语义 |
|---|---|---|---|
| `calls` | array of `{tool, args, expect}` | 否 | 结构化工具调用计划（形状与 tt_job.schema.json 的 calls 一致）。缺省 = 显式无计划 |
| `impasse` | bool | 否 | 显式死胡同标志。缺省 = `false` |

## 解析语义（parse_reasoning_output）

| 输入 | 结果 |
|---|---|
| `{"calls":[...], "impasse": true}` | `ReasoningSignal { calls, impasse: true }` → **Impass** |
| `{"calls":[...]}` / 裸数组 | `calls` 非空 → **NeedsTool**；`calls` 空 → **NoToolNeeded** |
| `{"impasse": true}`（无 calls） | `calls: [], impasse: true` → **Impass** |
| `{"impasse": false}`（无 calls） | `calls: [], impasse: false` → **NoToolNeeded**（纯对话回答） |
| 非法 JSON / calls 形状错误 | `Err` → **NoToolNeeded**（无计划，warn 记录） |

## 与 M1 `parse_llm_calls` 的关系

- `parse_llm_calls` 保持不动（pipeline stage 1 专用；`{"impasse":true}` 无 calls 时视为错误——pipeline 单趟哲学：解析失败不入账）。
- `parse_reasoning_output` 是 run_cycle 视角的超集：显式无计划对象（无 calls）合法化为空计划。

## 确定性约束

- 同一输入 + 同一 clock → 同一 calls / 同一 envelope（`job_id = FNV-1a(user_input)`，`created_at = clock → RFC3339`）。
- 无 UUID、无随机数；全部派生可回放。
