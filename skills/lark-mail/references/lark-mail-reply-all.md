---
name: lark-mail-reply-all
version: 1.1.0
description: "飞书邮箱：回复全部（自动聚合原 To/Cc 并排除自己）。如需创建回复全部草稿，优先使用 +reply-all --draft，而不是 +draft-create。"
metadata:
  category: "communication"
  requires:
    bins: ["larksuite-cli"]
  scopes: ["mail:user_mailbox.message:send", "mail:user_mailbox.message:modify", "mail:user_mailbox.message:readonly", "mail:user_mailbox:readonly", "mail:user_mailbox.message.address:read", "mail:user_mailbox.message.subject:read", "mail:user_mailbox.message.body:read"]
---

# mail +reply-all

> **前置条件：** 先阅读 [`../../lark-shared/SKILL.md`](../../lark-shared/SKILL.md) 了解认证、全局参数和安全规则。

回复全部会自动处理：
- 自动聚合原邮件发件人、原 To、原 Cc
- 自动排除当前用户地址，避免回给自己
- 自动维护会话头（`In-Reply-To` / `References`）

> **草稿模式**：如需创建回复全部草稿而不立即发送，使用 `--draft` 参数。**优先使用 `+reply-all --draft`，不要用 `+draft-create` 来创建回复全部草稿**，因为 `+reply-all` 会自动处理收件人聚合和会话头。

本 skill 对应 shortcut：`larksuite-cli mail +reply-all`。

## 安全约束

此工具发送邮件后对所有收件人可见，调用前**必须**先向用户确认：
1. 目标邮件
2. 回复正文
3. 最终收件人范围（To/Cc/Bcc）

**禁止**在用户未明确同意的情况下自行发送。

## 命令

```bash
# 回复全部（纯文本）
larksuite-cli mail +reply-all --message-id <邮件ID> --body '收到，已处理。'

# 回复全部并追加收件人/抄送
larksuite-cli mail +reply-all --message-id <邮件ID> --body '同步更新' --to lead@example.com --cc pm@example.com

# 从回复名单中排除某些地址
larksuite-cli mail +reply-all --message-id <邮件ID> --body '见上' --remove bot@example.com,noreply@example.com

# HTML 回复全部
larksuite-cli mail +reply-all --message-id <邮件ID> --body '<b>已完成</b>' --html

# 回复全部时插入内嵌图片（CID 为唯一标识符，可用随机字符串）
larksuite-cli mail +reply-all --message-id <邮件ID> --body '<img src="cid:a1b2c3d4e5f6a7b8c9d0"> 详见图示。' --html --inline '[{"cid":"a1b2c3d4e5f6a7b8c9d0","file_path":"./logo.png"}]'

# 创建回复全部草稿（不发送，返回 draft_id）
larksuite-cli mail +reply-all --message-id <邮件ID> --body '草稿内容' --draft

# Dry Run（仅打印请求，不发送）
larksuite-cli mail +reply-all --message-id <邮件ID> --body '测试' --dry-run
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--message-id <id>` | 是 | 被回复的邮件 ID |
| `--body <text>` | 是 | 回复正文（纯文本或 HTML） |
| `--from <email>` | 否 | 发件人邮箱地址（默认读取 user_mailboxes.profile.primary_email_address） |
| `--to <emails>` | 否 | 额外收件人，多个用逗号分隔（追加到自动聚合结果） |
| `--cc <emails>` | 否 | 额外抄送，多个用逗号分隔 |
| `--bcc <emails>` | 否 | 密送邮箱，多个用逗号分隔 |
| `--remove <emails>` | 否 | 从自动聚合结果中排除的邮箱，多个用逗号分隔 |
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

- 自动收件人规则：原发件人优先进入 To，原 To/Cc 进入 Cc。
- 地址会去重（大小写不敏感）。
- 自动排除当前用户地址（enterprise email），并叠加 `--remove` 规则。
- 通过 raw EML 维护会话头并尽量复用原 `thread_id`。

## 发送后跟进

回复发送成功后，询问用户是否需要将原邮件标记为已读。如果用户同意：

```bash
larksuite-cli mail user_mailbox.messages batch_modify_message --params '{"user_mailbox_id":"me"}' --data '{"message_ids":["<原邮件ID>"],"remove_label_ids":["UNREAD"]}'
```

## 相关命令

- `larksuite-cli mail +reply` — 仅回复发件人
- `larksuite-cli mail +forward` — 转发邮件
- `larksuite-cli mail user_mailbox.messages get` — 查看邮件详情
