---
name: lark-mail-draft-create
version: 1.0.0
description: "Lark Mail: Create a new email draft with recipients, subject, body, and optional attachments/inline images. Use when composing a brand-new email (not for reply or forward — those have dedicated shortcuts with --draft flag)."
metadata:
  category: "communication"
  requires:
    bins: ["larksuite-cli"]
  scopes: ["mail:user_mailbox.message:modify", "mail:user_mailbox:readonly"]
---

# mail +draft-create

> **Prerequisite:** Read [`../../lark-shared/SKILL.md`](../../lark-shared/SKILL.md) first for authentication, global flags, and safety rules.

Use this command to create a brand-new empty email draft from scratch. It is the right choice when you already know the final recipients, subject, and body.

Do not use this command for reply or forward workflows. Those flows should use dedicated reply or forward shortcuts that can create drafts without sending.

If you need to modify an existing draft, do not use this command. Use `larksuite-cli mail +draft-edit` instead.

## Safety Constraints

This command creates a draft — it does NOT send the email. The user can review, edit, or delete the draft in the Lark Mail UI before sending. Therefore:

- **Do NOT compose the email as text output and ask for confirmation.** When the user asks to "draft" / "起草" an email, directly call `+draft-create` to create the draft in Lark Mail.
- **If recipients are not specified, omit `--to`** — the draft will be created without recipients. The user can add them later.
- **Only ask for confirmation before creating if the user's request is genuinely ambiguous** (e.g., multiple possible interpretations of the content).
- **Sending** a draft is a separate action that requires explicit user confirmation.

## Commands

```bash
# Create a draft without recipients (user can add them later)
larksuite-cli mail +draft-create --subject 'Weekly report' --body 'Here is this week'\''s progress...'

# Create a plain-text draft with recipients
larksuite-cli mail +draft-create --to alice@example.com --subject 'Weekly report' --body 'Here is this week'\''s progress...'

# Create an HTML draft with an attachment and inline image (CID is a unique identifier, e.g. random hex)
larksuite-cli mail +draft-create --to alice@example.com --subject 'Preview' --body '<img src="cid:a1b2c3d4e5f6a7b8c9d0">' --html --attach ./report.pdf --inline '[{"cid":"a1b2c3d4e5f6a7b8c9d0","file_path":"./logo.png"}]'

# Dry run only
larksuite-cli mail +draft-create --to alice@example.com --subject 'Test' --body 'test' --dry-run
```

## Parameters

| Parameter | Required | Description |
|------|------|------|
| `--to <emails>` | No | Full To recipient list. Separate multiple addresses with commas. `Alice <alice@example.com>` format is supported. When omitted, the draft is created without recipients (they can be added later via `+draft-edit`). |
| `--subject <text>` | Yes | Final draft subject |
| `--body <text>` | Yes | Full email body. Written as plain text by default. If the content is HTML, also pass `--html` |
| `--from <email>` | No | Sender email address (acts as mailbox selector). If omitted, the current signed-in user's primary mailbox address is used |
| `--cc <emails>` | No | Full Cc recipient list, comma-separated |
| `--bcc <emails>` | No | Full Bcc recipient list, comma-separated |
| `--html` | No | Treat `--body` as HTML. If the HTML references `cid:...`, also add the matching inline images with `--inline` |
| `--attach <paths>` | No | Regular attachment file paths, comma-separated |
| `--inline <json>` | No | Inline images as a JSON array. Each entry requires `cid` (a unique identifier, e.g. a random hex string like `a1b2c3d4e5f6a7b8c9d0`) and `file_path`. Format: `'[{"cid":"a1b2c3d4e5f6a7b8c9d0","file_path":"./logo.png"}]'`. Must be used with `--html`; reference in body as `<img src="cid:a1b2c3d4e5f6a7b8c9d0">` |
| `--format <mode>` | No | Output format: `json` (default) / `pretty` / `table` / `ndjson` / `csv` |
| `--dry-run` | No | Print the request without executing it |

## Return Value

On success:

```json
{
  "ok": true,
  "data": {
    "draft_id": "draft-id"
  }
}
```

## Typical usage

### Compose a new email → create draft → review → send

```bash
# 1. Create draft
larksuite-cli mail +draft-create --to alice@example.com --subject 'Q1 Report' --body 'Please find the report attached.' --attach ./q1-report.pdf --format json

# 2. Review the draft in Lark Mail UI, or fetch it:
larksuite-cli mail user_mailbox.drafts get --params '{"user_mailbox_id":"me","draft_id":"<draft_id>"}'

# 3. Send the draft
larksuite-cli mail user_mailbox.drafts send --params '{"user_mailbox_id":"me","draft_id":"<draft_id>"}'
```

### Create an HTML draft with inline images

```bash
# CID is a unique identifier, e.g. random hex
larksuite-cli mail +draft-create \
  --to alice@example.com \
  --subject 'Newsletter' \
  --body '<h1>Hello</h1><img src="cid:c7d8e9f0a1b2c3d4e5f6">' \
  --html \
  --inline '[{"cid":"c7d8e9f0a1b2c3d4e5f6","file_path":"./banner.png"}]'
```

## Related Commands

- `larksuite-cli mail +draft-edit` — edit an existing draft
- `larksuite-cli mail user_mailbox.drafts send` — send an existing draft
- `larksuite-cli mail user_mailbox.drafts get` — fetch a draft
- `larksuite-cli mail +reply` / `+reply-all` / `+forward` — send reply or forward messages directly
