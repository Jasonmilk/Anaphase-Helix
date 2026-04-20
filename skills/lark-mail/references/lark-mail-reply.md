---
name: lark-mail-reply
version: 1.2.0
description: "飞书邮箱：回复邮件（自动设置 Re: 主题、收件人和 In-Reply-To/References 会话头）。如需创建回复草稿，优先使用 +reply --draft，而不是 +draft-create。"
metadata:
  category: "communication"
  requires:
    bins: ["larksuite-cli"]
  scopes: ["mail:user_mailbox.message:send", "mail:user_mailbox.message:modify", "mail:user_mailbox.message:readonly", "mail:user_mailbox:readonly", "mail:user_mailbox.message.address:read", "mail:user_mailbox.message.subject:read", "mail:user_mailbox.message.body:read"]
---

# mail +reply

> **前置条件：** 先阅读 [`../../lark-shared/SKILL.md`](../../lark-shared/SKILL.md) 了解认证、全局参数和安全规则。

回复指定邮件，自动处理：
- 主题前缀 `Re: `（已含常见回复前缀时不重复叠加）
- 默认收件人为原邮件发件人
- RFC 2822 会话头（`In-Reply-To` / `References`）维护邮件会话

> **草稿模式**：如需创建回复草稿而不立即发送，使用 `--draft` 参数。**优先使用 `+reply --draft`，不要用 `+draft-create` 来创建回复草稿**，因为 `+reply` 会自动处理主题、收件人和会话头。

本 skill 对应 shortcut：`larksuite-cli mail +reply`，内部步骤：
1. `GET /open-apis/mail/v1/user_mailboxes/me/messages/{message_id}` — 获取原邮件元数据
2. `GET /open-apis/mail/v1/user_mailboxes/me/profile` — 获取邮箱主地址（`primary_email_address`，填入默认 From 头）
3. `POST /open-apis/mail/v1/user_mailboxes/me/drafts` — 创建草稿
4. `POST /open-apis/mail/v1/user_mailboxes/me/drafts/{draft_id}/send` — 发送草稿（`--draft` 模式跳过此步骤）

## 安全约束

此工具发送邮件后对方可见，调用前**必须**先向用户确认：
1. 回复的目标邮件
2. 回复内容

**禁止**在用户未明确同意的情况下自行回复邮件。

## 命令

```bash
# 回复一封邮件（纯文本）
larksuite-cli mail +reply --message-id <邮件ID> --body '收到，谢谢！'

# 回复并追加收件人/抄送
larksuite-cli mail +reply --message-id <邮件ID> --body '已处理' --to lead@example.com --cc colleague@example.com

# 回复 HTML 正文
larksuite-cli mail +reply --message-id <邮件ID> --body '<b>已收到</b>，稍后跟进。' --html

# 回复时插入内嵌图片（CID 为唯一标识符，可用随机字符串）
larksuite-cli mail +reply --message-id <邮件ID> --body '<img src="cid:a1b2c3d4e5f6a7b8c9d0"> 详见图示。' --html --inline '[{"cid":"a1b2c3d4e5f6a7b8c9d0","file_path":"./logo.png"}]'

# 指定发件人地址
larksuite-cli mail +reply --message-id <邮件ID> --body '收到' --from me@example.com

# 创建回复草稿（不发送，返回 draft_id）
larksuite-cli mail +reply --message-id <邮件ID> --body '草稿内容' --draft

# Dry Run（仅打印请求，不发送）
larksuite-cli mail +reply --message-id <邮件ID> --body '测试' --dry-run
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--message-id <id>` | 是 | 被回复的邮件 ID |
| `--body <text>` | 是 | 回复正文（纯文本或 HTML） |
| `--from <email>` | 否 | 发件人邮箱地址（默认读取 user_mailboxes.profile.primary_email_address） |
| `--to <emails>` | 否 | 额外收件人，多个用逗号分隔（追加到原发件人） |
| `--cc <emails>` | 否 | 抄送邮箱，多个用逗号分隔 |
| `--bcc <emails>` | 否 | 密送邮箱，多个用逗号分隔 |
| `--html` | 否 | 将 `--body` 视为 HTML 片段（引用块格式自动跟随原邮件内容，无需指定） |
| `--attach <paths>` | 否 | 附件文件路径，多个用逗号分隔 |
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

## 实现说明

### 会话维护

本 shortcut 通过 raw EML 方式发送，包含标准 RFC 2822 会话头：

```
In-Reply-To: <原邮件smtp_message_id>
References:  <原邮件references + smtp_message_id>
```

若原邮件有 `thread_id`，发送时会一并传入，确保回复归入同一会话。

### 收件人与引用

- 默认回复给原邮件发件人（`head_from`）
- `--to` 会在默认收件人基础上追加
- 自动拼接引用块（纯文本或 HTML）

## 发送后跟进

回复发送成功后，询问用户是否需要将原邮件标记为已读。如果用户同意：

```bash
larksuite-cli mail user_mailbox.messages batch_modify_message --params '{"user_mailbox_id":"me"}' --data '{"message_ids":["<原邮件ID>"],"remove_label_ids":["UNREAD"]}'
```

## 注意事项

- 需要已登录（`larksuite-cli auth login --scope "mail:user_mailbox.message:send mail:user_mailbox.message:readonly mail:user_mailbox:readonly"`）且具备写/读邮件权限
- 邮件 ID 可从 `larksuite-cli mail user_mailbox.messages list` 获取
- `--bcc` 仅在发送链路中生效，通常不会在收件方看到

## 相关命令

- `larksuite-cli mail user_mailbox.messages list` — 列出邮件
- `larksuite-cli mail user_mailbox.messages get` — 读取邮件详情
- `larksuite-cli mail +reply-all` — 回复全部
- `larksuite-cli mail +forward` — 转发邮件
