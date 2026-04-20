# im +messages-send


Send a message to a group chat or a direct message conversation.

This skill maps to the shortcut: `lark-cli im +messages-send` (internally calls `POST /open-apis/im/v1/messages`).

## Safety Constraints

Messages sent by this tool are visible to other people. Before calling it, you **must** confirm with the user:

1. The recipient (which person or which group)
2. The message content

**Do not** send messages without explicit user approval.

## Commands

```bash
# Send plain text (--text is recommended; it is wrapped into JSON automatically)
lark-cli im +messages-send --chat-id oc_xxx --text "Hello"

# Equivalent manual JSON
lark-cli im +messages-send --chat-id oc_xxx --content '{"text":"Hello"}'

# Send to a direct message (pass open_id)
lark-cli im +messages-send --user-id ou_xxx --text "Hello"

# Send a rich-text message
lark-cli im +messages-send --chat-id oc_xxx --msg-type post --content '{"zh_cn":{"title":"Title","content":[[{"tag":"text","text":"Body"}]]}}'

# Send an image by image_key
lark-cli im +messages-send --chat-id oc_xxx --image img_xxx

# Send a file by URL (uploaded automatically before sending)
lark-cli im +messages-send --chat-id oc_xxx --file https://example.com/report.pdf

# Send a video by URL (--video-cover is required as the cover)
lark-cli im +messages-send --chat-id oc_xxx --video https://example.com/demo.mp4 --video-cover img_xxx

# Send audio by URL
lark-cli im +messages-send --chat-id oc_xxx --audio https://example.com/voice.opus

# Use an idempotency key (same key sends only once within 1 hour)
lark-cli im +messages-send --chat-id oc_xxx --text "Hello" --idempotency-key my-unique-id

# Preview the request without executing it
lark-cli im +messages-send --chat-id oc_xxx --text "Test" --dry-run
```

## Parameters

| Parameter | Required | Description |
|------|------|------|
| `--chat-id <id>` | One of two | Group chat ID (`oc_xxx`) |
| `--user-id <id>` | One of two | User open_id (`ou_xxx`) for direct messages |
| `--text <string>` | One of seven content options | Plain text message (automatically wrapped as `{"text":"..."}` JSON) |
| `--markdown <string>` | One of seven content options | Markdown text (auto-wrapped as post format with style optimization; image URLs auto-resolved) |
| `--content <json>` | One of seven content options | Message content JSON string; format depends on `msg_type` |
| `--image <path\|key>` | One of seven content options | Image URL or `image_key` (`img_xxx`) |
| `--file <path\|key>` | One of seven content options | File URL or `file_key` (`file_xxx`) |
| `--video <path\|key>` | One of seven content options | Video URL or `file_key`. **Must be paired with `--video-cover`** |
| `--video-cover <path\|key>` | **Required with `--video`** | Video cover URL or `image_key` (`img_xxx`) |
| `--audio <path\|key>` | One of seven content options | Audio URL or `file_key` |
| `--msg-type <type>` | No | Message type (default `text`): `text`, `post`, `image`, `file`, `audio`, `media`, `interactive`, `share_chat`, `share_user`. Automatically set when using `--text`/`--image`/`--file`/`--video`/`--audio` |
| `--idempotency-key <key>` | No | Idempotency key; the same key sends only one message within 1 hour |
| `--dry-run` | No | Print the request only, do not execute it |

> **Mutual exclusivity rule:** `--text`, `--markdown`, `--content`, and `--image`/`--file`/`--video`/`--audio` cannot be used together. Media flags are also mutually exclusive with each other.
>
> **Video cover rule:** `--video` **must** be accompanied by `--video-cover`. Omitting `--video-cover` when using `--video` will fail validation. `--video-cover` cannot be used without `--video`.

## `content` Format Reference

| `msg_type` | Example `content` |
|----------|-------------|
| `text` | `{"text":"Hello <at user_id=\"ou_xxx\">name</at>"}` |
| `post` | `{"zh_cn":{"title":"Title","content":[[{"tag":"text","text":"Body"}]]}}` |
| `image` | `{"image_key":"img_xxx"}` |
| `file` | `{"file_key":"file_xxx"}` |
| `audio` | `{"file_key":"file_xxx"}` |
| `media` | `{"file_key":"file_xxx","image_key":"img_xxx"}` (video; `image_key` is the cover from `--video-cover` — **required**) |
| `share_chat` | `{"chat_id":"oc_xxx"}` |
| `share_user` | `{"user_id":"ou_xxx"}` |
| `interactive` | Card JSON (see Feishu interactive card documentation) |

## Return Value

```json
{
  "message_id": "om_xxx",
  "chat_id": "oc_xxx",
  "create_time": "1234567890"
}
```

## @Mention Format (text / post)

- @specific user: `<at user_id="ou_xxx">name</at>`
- @all: `<at user_id="all"></at>`

## Notes

- `--chat-id` and `--user-id` are mutually exclusive; you must provide exactly one
- `--content` must be a valid JSON string
- `--image`/`--file`/`--video`/`--audio` support URL or file_key
- If the provided value starts with `img_` or `file_`, it is treated as an existing key and used directly
- When using `--video`, `--video-cover` is **required** as the video cover (`image_key`). Omitting `--video-cover` with `--video` will produce a validation error. `--video-cover` cannot be used without `--video`
- Failures return an error code and message
