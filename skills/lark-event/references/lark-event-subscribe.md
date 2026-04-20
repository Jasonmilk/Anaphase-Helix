
# event +subscribe

> **前置条件：** 先阅读 [`../lark-shared/SKILL.md`](../../lark-shared/SKILL.md) 了解认证、全局参数和安全规则。

通过 WebSocket 长连接实时接收飞书事件推送（消息、日历变更等），输出 NDJSON 到 stdout。

**认证**：仅需 App ID + App Secret（`larksuite-cli config init`），不需要用户登录。

**平台侧配置**（必须在飞书开放平台控制台完成）：
1. 事件与回调 → 订阅方式 → 选择「使用长连接接收事件」
2. 添加需要的事件（如 `im.message.receive_v1`）
3. 开通对应权限（如 `im:message:receive_as_bot`）

## 命令

```bash
# 监听所有已订阅事件（catch-all 模式）
larksuite-cli event +subscribe

# 只监听特定事件类型
larksuite-cli event +subscribe --event-types im.message.receive_v1

# 监听多个事件类型
larksuite-cli event +subscribe --event-types im.message.receive_v1,calendar.calendarEvent.changed_v1

# 正则过滤（客户端侧）
larksuite-cli event +subscribe --filter "^im\."

# Agent 友好格式（解析 content、去噪音字段）
larksuite-cli event +subscribe --event-types im.message.receive_v1 --compact --quiet

# 格式化 JSON 输出
larksuite-cli event +subscribe --json

# 事件写入文件
larksuite-cli event +subscribe --output-dir ./events

# 检查配置（不连接）
larksuite-cli event +subscribe --dry-run
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--event-types <types>` | 否 | 逗号分隔的事件类型，默认 catch-all（接收所有事件） |
| `--filter <regex>` | 否 | 客户端正则过滤 event_type，可与 `--event-types` 叠加 |
| `--compact` | 否 | Agent 友好输出：解析 content JSON、去除 token/tenant_key 等噪音 |
| `--json` | 否 | 格式化 JSON 输出（默认 NDJSON 单行） |
| `--output-dir <dir>` | 否 | 每个事件写入独立文件：`{type}_{id}_{ts}.json` |
| `--quiet` | 否 | 静默模式，不输出状态信息到 stderr |
| `--dry-run` | 否 | 仅打印配置，不连接 WebSocket |

## 输出格式

### 默认（原始 NDJSON）

每行一个事件，包含所有字段：

```json
{"schema":"2.0","event_id":"xxx","event_type":"im.message.receive_v1","app_id":"cli_xxx","message":{"chat_id":"oc_xxx","content":"{\"text\":\"你好\"}","message_id":"om_xxx","message_type":"text"},"sender":{"sender_id":{"open_id":"ou_xxx"},"sender_type":"user"}}
```

### `--compact`（Agent 友好）

解析双重编码的 content，提取关键字段，去除噪音：

```json
{"event_type":"im.message.receive_v1","message_id":"om_xxx","chat_id":"oc_xxx","chat_type":"p2p","message_type":"text","text":"你好","sender_id":"ou_xxx","create_time":"1773491924409"}
```

**`--compact` 对 `im.message.receive_v1` 的处理**：
- `content: "{\"text\":\"你好\"}"` → `text: "你好"`（解析双重 JSON）
- `sender.sender_id.open_id` → `sender_id`（扁平化）
- 去除 `schema`、`token`、`tenant_key`、`app_id`

Agent 管道场景**始终使用 `--compact --quiet`**。

## 常用事件类型

| 事件类型 | 说明 | 所需权限 |
|---------|------|---------|
| `im.message.receive_v1` | 接收消息 | `im:message:receive_as_bot` |
| `im.message.message_read_v1` | 消息已读 | `im:message:receive_as_bot` |
| `im.chat.member.bot.added_v1` | Bot 被加入群 | `im:chat:readonly` |
| `calendar.calendarEvent.changed_v1` | 日程变更 | `calendar:calendar:readonly` |
| `contact.user.updated_v6` | 用户信息变更 | `contact:user.base:readonly` |

完整列表参见[飞书事件列表文档](https://open.feishu.cn/document/server-docs/event-subscription-guide/event-list)。

## Agent 管道用法

### 监听消息并用 Claude 回复

```bash
larksuite-cli event +subscribe \
  --event-types im.message.receive_v1 --compact --quiet \
  | while IFS= read -r line; do
      text=$(echo "$line" | jq -r '.text // empty')
      message_id=$(echo "$line" | jq -r '.message_id // empty')
      [[ -z "$text" ]] && continue

      # Claude 生成回答
      answer=$(claude -p "简洁回答: $text" < /dev/null 2>/dev/null)

      # Bot 身份回复
      reply_data=$(jq -n --arg t "$answer" '{msg_type:"text",content:({text:$t}|tojson)}')
      larksuite-cli api POST "/open-apis/im/v1/messages/$message_id/reply" \
        --data "$reply_data" --as bot --format data
    done
```

### 监听消息并记录到飞书文档

```bash
larksuite-cli event +subscribe \
  --event-types im.message.receive_v1 --compact --quiet \
  | while IFS= read -r line; do
      text=$(echo "$line" | jq -r '.text // empty')
      [[ -z "$text" ]] && continue

      larksuite-cli docs +update --doc "DOC_URL" --mode append --markdown "- $text"
    done
```

## 注意事项

- **事件必须在开放平台控制台配置**，CLI 无法动态订阅事件类型
- `--event-types` 和 `--filter` 是**客户端过滤**，不减少服务端推送量
- WebSocket 连接支持自动重连（SDK 内置），无需手动处理
- `Ctrl+C` 优雅退出，会打印接收事件总数
- Bot 回复消息用 `larksuite-cli api ... --as bot`，不需要用户登录

## 参考

- [lark-im](../../lark-im/SKILL.md) — 消息相关命令
- [lark-doc-update](../../lark-doc/references/lark-doc-update.md) — 更新飞书文档
- [lark-shared](../../lark-shared/SKILL.md) — 认证和全局参数
