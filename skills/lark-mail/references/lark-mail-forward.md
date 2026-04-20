---
name: lark-mail-forward
version: 1.1.0
description: "飞书邮箱：转发邮件（自动组装 Forwarded message 区块）。如需创建转发草稿，优先使用 +forward --draft，而不是 +draft-create。"
metadata:
  category: "communication"
  requires:
    bins: ["larksuite-cli"]
  scopes: ["mail:user_mailbox.message:send", "mail:user_mailbox.message:modify", "mail:user_mailbox.message:readonly", "mail:user_mailbox:readonly", "mail:user_mailbox.message.address:read", "mail:user_mailbox.message.subject:read", "mail:user_mailbox.message.body:read"]
---

# mail +forward

> **前置条件：** 先阅读 [`../../lark-shared/SKILL.md`](../../lark-shared/SKILL.md) 了解认证、全局参数和安全规则。

转发指定邮件，自动处理：
- 主题前缀 `Fwd: `（已含前缀时不重复）
- 自动拼接标准 “Forwarded message” 区块（From/Date/Subject/To + 原文）
- 支持纯文本和 HTML 转发

> **草稿模式**：如需创建转发草稿而不立即发送，使用 `--draft` 参数。**优先使用 `+forward --draft`，不要用 `+draft-create` 来创建转发草稿**，因为 `+forward` 会自动处理原邮件附件和转发区块。

本 skill 对应 shortcut：`larksuite-cli mail +forward`。

## 安全约束

此工具会把原邮件内容转发给新收件人，调用前**必须**先向用户确认：
1. 被转发邮件
2. 新收件人（To/Cc/Bcc）
3. 是否附加说明文字（`--body`）

**禁止**在用户未明确同意的情况下转发邮件。

## 命令

```bash
# 转发邮件（纯文本）
larksuite-cli mail +forward --message-id <邮件ID> --to alice@example.com

# 转发并附加说明 + 抄送
larksuite-cli mail +forward --message-id <邮件ID> --to alice@example.com --cc bob@example.com --body 'FYI，请看下面原邮件。'

# HTML 转发
larksuite-cli mail +forward --message-id <邮件ID> --to alice@example.com --body '<b>请参考</b>' --html

# 转发时插入内嵌图片（CID 为唯一标识符，可用随机字符串）
larksuite-cli mail +forward --message-id <邮件ID> --to alice@example.com --body '<img src="cid:a1b2c3d4e5f6a7b8c9d0"> 详见图示。' --html --inline '[{"cid":"a1b2c3d4e5f6a7b8c9d0","file_path":"./logo.png"}]'

# 创建转发草稿（不发送，返回 draft_id）
larksuite-cli mail +forward --message-id <邮件ID> --to alice@example.com --draft

# Dry Run（仅打印请求，不发送）
larksuite-cli mail +forward --message-id <邮件ID> --to alice@example.com --dry-run
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--message-id <id>` | 是 | 被转发的邮件 ID |
| `--to <emails>` | 是 | 收件人邮箱，多个用逗号分隔 |
| `--body <text>` | 否 | 转发时附加的说明文字 |
| `--from <email>` | 否 | 发件人邮箱地址（默认读取 user_mailboxes.profile.primary_email_address） |
| `--cc <emails>` | 否 | 抄送邮箱，多个用逗号分隔 |
| `--bcc <emails>` | 否 | 密送邮箱，多个用逗号分隔 |
| `--html` | 否 | 将 `--body` 视为 HTML 片段（转发区块格式自动跟随原邮件内容，无需指定） |
| `--attach <paths>` | 否 | 附件文件路径，多个用逗号分隔（追加在原邮件附件之后） |
| `--inline <json>` | 否 | 内嵌图片 JSON 数组，每项包含 `cid`（唯一标识符，可用随机十六进制字符串，如 `a1b2c3d4e5f6a7b8c9d0`）和 `file_path`。格式：`'[{"cid":"a1b2c3d4e5f6a7b8c9d0","file_path":"./logo.png"}]'`。须配合 `--html` 使用，在 body 中用 `<img src="cid:...">` 引用 |
| `--draft` | 否 | 创建草稿而不发送，返回 `draft_id` |
| `--dry-run` | 否 | 仅打印请求，不执行 |

## 返回值

发送成功：
```json
{
  "ok": true,
  "data": {
    "message_id": "邮件ID",
    "thread_id":  "会话ID"
  }
}
```

`--draft` 模式：
```json
{
  "ok": true,
  "data": {
    "draft_id": "草稿ID"
  }
}
```

## 转发整个会话

`+forward` 操作的是单封邮件（`--message-id`），但转发整个会话时应 forward **会话中最后一条消息**，因为邮件客户端会将完整的回复链嵌套在最新一条中。典型流程：

```bash
# 1. 用 +triage 或 +thread 找到会话
larksuite-cli mail +thread --thread-id <THREAD_ID> --html=false --format json

# 2. 取最后一条消息的 message_id
#    messages 按时间升序排列，最后一条 = messages[-1].message_id

# 3. 转发该消息
larksuite-cli mail +forward --message-id <最后一条的message_id> --to recipient@example.com --body '请过目'
```

## 实现说明

- 自动拉取原邮件后构建转发内容。
- 纯文本模式下会生成标准转发头块并附上原文文本。
- HTML 模式下会生成结构化转发块并尽量保留原 HTML 正文。

## 发送后跟进

转发发送成功后，询问用户是否需要将原邮件标记为已读。如果用户同意：

```bash
larksuite-cli mail user_mailbox.messages batch_modify_message --params '{"user_mailbox_id":"me"}' --data '{"message_ids":["<原邮件ID>"],"remove_label_ids":["UNREAD"]}'
```

## 相关命令

- `larksuite-cli mail +send` — 发送新邮件
- `larksuite-cli mail +reply` — 回复邮件
- `larksuite-cli mail user_mailbox.messages get` — 查看邮件详情
