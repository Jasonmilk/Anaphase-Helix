# im reactions.list

> **Prerequisite:** Read [`../lark-shared/SKILL.md`](../../lark-shared/SKILL.md) first to understand authentication, global parameters, and safety rules.

Query reactions on a specific message. This is a raw API reference for `im reactions list`, useful when you need the exact reaction records, pagination fields, and response structure.

This command calls `GET /open-apis/im/v1/messages/{message_id}/reactions`.

> **Important:** This raw API command accepts request input through `--params '<json>'`. It does not expose typed flags such as `--message-id` or `--reaction-type` directly.

## Commands

```bash
# Inspect the request/response schema first
larksuite-cli schema im.reactions.list

# List all reactions on a message
larksuite-cli im reactions list --params '{"message_id":"om_xxx"}'

# Only list a specific emoji type
larksuite-cli im reactions list --params '{"message_id":"om_xxx","reaction_type":"SMILE"}'

# Pagination
larksuite-cli im reactions list --params '{"message_id":"om_xxx","page_size":50}'
larksuite-cli im reactions list --params '{"message_id":"om_xxx","page_token":"<PAGE_TOKEN>"}'

# Control returned operator ID type when operator is a user
larksuite-cli im reactions list --params '{"message_id":"om_xxx","user_id_type":"open_id"}'
```

## Request Parameters (`--params` JSON)

| Parameter | Required | Description |
|------|------|------|
| `message_id` | Yes | Message ID (`om_xxx`). This is the message whose reactions you want to query |
| `reaction_type` | No | Filter by a specific emoji type such as `SMILE` or `LAUGH`. Omit it to return all reactions on the message |
| `page_size` | No | Number of records per page. The API default is 20 |
| `page_token` | No | Pagination token from the previous page |
| `user_id_type` | No | Returned user ID type when the operator is a user: `open_id`, `union_id`, or `user_id` |

Example:

```bash
larksuite-cli im reactions list --params '{"message_id":"om_xxx","reaction_type":"SMILE","page_size":20}'
```

## Response Shape

```json
{
  "items": [
    {
      "reaction_id": "ZCaCIjUBVVWSrm5L-3ZTw_xxx",
      "operator": {
        "operator_id": "ou_xxx",
        "operator_type": "user"
      },
      "action_time": "1663054162546",
      "reaction_type": {
        "emoji_type": "SMILE"
      }
    }
  ],
  "has_more": true,
  "page_token": "YhljsPiGfUgnVAg9urvRFd-BvSqRLxxxx"
}
```

## Top-Level Fields

| Field | Type | Meaning |
|------|------|------|
| `items` | `array<object>` | Reaction records for the current page |
| `has_more` | `boolean` | Whether more pages are available |
| `page_token` | `string` | The token for the next page. Only meaningful when `has_more=true` |

## `items[]` Field Definitions

Each `items[]` element represents one reaction record on the target message.

| Field | Type | Meaning |
|------|------|------|
| `reaction_id` | `string` | Unique ID of this reaction record. Use it with `im reactions delete` when deleting a reaction |
| `operator` | `object` | Identity of the user or app that added the reaction |
| `action_time` | `string` | Unix timestamp in milliseconds indicating when the reaction was added |
| `reaction_type` | `object` | Reaction payload. Currently the key field is `emoji_type` |

## `operator` Field Definitions

| Field | Type | Meaning |
|------|------|------|
| `operator.operator_id` | `string` | Identifier of the operator. If `operator_type=user`, the returned ID type follows `--user-id-type`; if `operator_type=app`, this is the app ID |
| `operator.operator_type` | `string` | Operator identity type: `user` or `app` |

## `reaction_type` Field Definitions

| Field | Type | Meaning |
|------|------|------|
| `reaction_type.emoji_type` | `string` | Emoji enum value, such as `SMILE` or `LAUGH`. Refer to Feishu's emoji type documentation when constructing filters |

## Interpretation Notes

- This API returns **reaction records**, not aggregated counts. If 3 different people reacted with the same emoji, you will usually see 3 records in `items`
- `reaction_id` is the stable handle for follow-up delete operations
- `action_time` is a millisecond timestamp string; convert it before displaying local time
- `page_token` is part of the response contract. Do not guess or synthesize it
- `--user-id-type` only affects `operator.operator_id` when the operator is a human user
- The CLI also supports pagination helpers such as `--page-all`, but the actual API request fields are still passed through `--params`

## Common Follow-Up Actions

```bash
# Delete a specific reaction after reading reaction_id from list output
larksuite-cli im reactions delete --params '{"message_id":"om_xxx","reaction_id":"ZCaCIjUBVVWSrm5L-3ZTw_xxx"}'

# Fetch messages first if you only know the chat and need to locate a message_id
larksuite-cli im +chat-messages-list --chat oc_xxx
```

## References

- [lark-im](../SKILL.md) - all IM commands
- [lark-shared](../../lark-shared/SKILL.md) - authentication and global parameters
