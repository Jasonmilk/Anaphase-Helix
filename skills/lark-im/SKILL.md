---
name: lark-im
version: 1.0.0
description: "飞书即时通讯：收发消息和管理群聊。发送和回复消息、搜索聊天记录、管理群聊成员、管理表情回复。当用户需要发消息、查看或搜索聊天记录、查看群成员时使用。"
metadata:
  requires:
    tools: ["lark_cli_exec"]
---

# im (v1)

## Core Concepts

- **Message**: A single message in a chat, identified by `message_id` (om_xxx). Supports types: text, post, image, file, audio, video, sticker, interactive (card), share_chat, share_user, merge_forward, etc.
- **Chat**: A group chat or P2P conversation, identified by `chat_id` (oc_xxx).
- **Thread**: A reply thread under a message, identified by `thread_id` (om_xxx or omt_xxx).
- **Reaction**: An emoji reaction on a message.

## Resource Relationships

```
Chat (oc_xxx)
├── Message (om_xxx)
│   ├── Thread (reply thread)
│   ├── Reaction (emoji)
│   └── Resource (image / file / video / audio)
└── Member (user / bot)
```

## ID 获取指引

本技能的多个命令需要 **用户 open_id**（`ou_xxx`）或 **群聊 chat_id**（`oc_xxx`）作为入参。当上下文中没有现成的 ID，需要通过搜索来获取时：

- **搜索用户获取 open_id**：参考 [lark-contact](../lark-contact/SKILL.md) 技能，使用 `lark-cli contact +search-user` 按姓名或邮箱搜索用户。
- **搜索群聊获取 chat_id**：使用本技能的 `lark-cli im +chat-search` 按群名关键词搜索群聊。

## Important Notes

### Card Messages (Interactive)

Card messages (`interactive` type) are not yet supported for compact conversion in event subscriptions. The raw event data will be returned instead, with a hint printed to stderr.

## Shortcuts（推荐优先使用）

Shortcut 是对常用操作的高级封装（`lark-cli im +<verb> [flags]`）。有 Shortcut 的操作优先使用。

| Shortcut | 说明 |
|----------|------|
| [`+chat-create`](references/lark-im-chat-create.md) | Create a group chat; creates private/public chats, invites users/bots |
| [`+chat-messages-list`](references/lark-im-chat-messages-list.md) | List messages in a chat or P2P conversation; accepts --chat-id or --user-id, resolves P2P chat_id, supports time range/sort/pagination |
| [`+chat-search`](references/lark-im-chat-search.md) | Search visible group chats by keyword and/or member open_ids (e.g. look up chat_id by group name); supports member/type filters, sorting, and pagination |
| [`+chat-update`](references/lark-im-chat-update.md) | Update group chat name or description; updates a chat's name or description |
| [`+messages-mget`](references/lark-im-messages-mget.md) | Batch get messages by IDs; fetches up to 50 om_ message IDs, formats sender names, expands thread replies |
| [`+messages-reply`](references/lark-im-messages-reply.md) | Reply to a message (supports thread replies); supports text/markdown/post/media replies, reply-in-thread, idempotency key |
| [`+messages-resources-download`](references/lark-im-messages-resources-download.md) | Download images/files from a message; retrieves image/file resources by message-id and file-key |
| [`+messages-search`](references/lark-im-messages-search.md) | Search messages across chats (supports keyword, sender, time range filters); filters by chat/sender/attachment/time, enriches results via mget and chats batch_query |
| [`+messages-send`](references/lark-im-messages-send.md) | Send a message to a chat or direct message; sends to chat-id or user-id with text/markdown/post/media, supports idempotency key |
| [`+threads-messages-list`](references/lark-im-threads-messages-list.md) | List messages in a thread; accepts om_/omt_ input, resolves message IDs to thread_id, supports sort/pagination |

## API Resources

```bash
lark-cli schema im.<resource>.<method>   # 调用 API 前必须先查看参数结构
lark-cli im <resource> <method> [flags] # 调用 API
```

> **重要**：使用原生 API 时，必须先运行 `schema` 查看 `--data` / `--params` 参数结构，不要猜测字段格式。

### chats

  - `get` — 获取群信息。 The caller must be in the target chat to get full details, and must belong to the same tenant for internal chats.
  - `link` — 获取群分享链接。The caller must be in the target chat, must be an owner or admin when chat sharing is restricted to owners/admins, and must belong to the same tenant for internal chats.
  - `list` — 获取用户所在的群列表。
  - `update` — 更新群信息。

### chat.members

  - `create` — 将用户或机器人拉入群聊。The caller must be in the target chat; for internal chats the operator must belong to the same tenant; if only owners/admins can add members, the caller must be an owner/admin.
  - `get` — 获取群成员列表。The caller must be in the target chat and must belong to the same tenant for internal chats.

### messages

  - `delete` — 撤回消息。The caller must be in the chat; to revoke another user's group message, the caller must be the owner, an admin, or the creator.

### reactions

  - `batch_query` — 批量获取消息表情。[Must-read](references/lark-im-reactions.md)
  - `create` — 添加消息表情回复。The caller must be in the conversation that contains the message.[Must-read](references/lark-im-reactions.md)
  - `delete` — 删除消息表情回复。The caller must be in the conversation that contains the message, and can only delete reactions added by itself.[Must-read](references/lark-im-reactions.md)
  - `list` — 获取消息表情回复。The caller must be in the conversation that contains the message.[Must-read](references/lark-im-reactions.md)

### pins

  - `create` — Pin 消息。
  - `delete` — 移除 Pin 消息。
  - `list` — 获取群内 Pin 消息。
