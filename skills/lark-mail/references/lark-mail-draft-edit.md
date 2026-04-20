---
name: lark-mail-draft-edit
version: 1.0.0
description: "Lark Mail: Edit an existing email draft. Supports direct flags for common final-state edits (subject, recipients, body) and --patch-file for advanced incremental operations (add/remove recipients, attachment/inline-image edits, header edits)."
metadata:
  category: "communication"
  requires:
    bins: ["larksuite-cli"]
  scopes: ["mail:user_mailbox.message:modify", "mail:user_mailbox.message:readonly", "mail:user_mailbox:readonly"]
---

# mail +draft-edit

> **Prerequisite:** Read [`../../lark-shared/SKILL.md`](../../lark-shared/SKILL.md) first for authentication, global flags, and safety rules.

Use this command to edit an existing email draft. The command reads the current raw EML, applies a minimal patch, and writes the updated draft back.

Prefer direct flags for common final-state edits:
- `--set-subject`
- `--set-to`
- `--set-cc`
- `--set-bcc`
- `--set-body`

If you need complex edits, such as incremental recipient changes, header edits, attachment edits, or inline-image edits, use `--patch-file` instead.

Decision rule:

- Use direct flags when you already know the complete final state you want to write back, for example the final subject, full To list, or final body content.
- Use `--patch-file` when you need incremental changes, for example add or remove one recipient, remove one attachment by `cid` or `part_id`, replace one inline image, or edit headers.

### Body edit: coupled plain+HTML drafts

When a draft contains both `text/plain` and `text/html` parts, they form a coupled pair. Both `--set-body` and `set_body` patch op update the HTML body and auto-regenerate the plain-text summary. In this case, always pass HTML as input because the original main body is `text/html`.

| Situation | Action |
|------|------|
| Draft has only plain text or only HTML | `--set-body` / `set_body` — pass content as-is |
| Draft has both `text/plain` and `text/html` | `--set-body` / `set_body` — pass **HTML** input; plain-text is regenerated |

## Safety Constraints

This command updates a real draft. Before calling it, you must confirm with the user:
1. The draft ID
2. The final recipient scope (To/Cc/Bcc)
3. The final subject and body
4. Whether attachments, inline images, or other advanced edits are needed

## Commands

```bash
# Set the final draft state directly
larksuite-cli mail +draft-edit --draft-id <draft-id> --set-subject 'Updated subject' --set-to alice@example.com,bob@example.com --set-body 'New body'

# Update the draft body
larksuite-cli mail +draft-edit --draft-id <draft-id> --set-body 'Updated body'

# Inspect a draft (read-only) — returns projection with attachments_summary and inline_summary showing part_id and cid for each part
larksuite-cli mail +draft-edit --draft-id <draft-id> --inspect

# Print the patch template
larksuite-cli mail +draft-edit --print-patch-template

# Use patch-file for advanced edits
larksuite-cli mail +draft-edit --draft-id d_xxx --patch-file ./patch.json

# Dry run only
larksuite-cli mail +draft-edit --draft-id <draft-id> --set-subject 'Test' --dry-run
```

## Common Parameters

| Parameter | Required | Description |
|------|------|------|
| `--draft-id <id>` | Yes | Target draft ID. It can be omitted only when `--print-patch-template` is used by itself |
| `--set-subject <text>` | No | Replace the subject with this final value |
| `--set-to <emails>` | No | Replace the entire To recipient list with the addresses provided here |
| `--set-cc <emails>` | No | Replace the entire Cc recipient list with the addresses provided here |
| `--set-bcc <emails>` | No | Replace the entire Bcc recipient list with the addresses provided here |
| `--set-body <text>` | No | Replace the main draft body. See [Body edit: coupled plain+HTML drafts](#body-edit-coupled-plainhtml-drafts) for handling drafts with both text/plain and text/html |
| `--patch-file <path>` | No | Advanced edit entry point. First run `--print-patch-template` to inspect the expected JSON structure, then pass a typed patch JSON file for incremental recipient edits, header edits, attachment changes, and inline-image changes |
| `--print-patch-template` | No | Print the JSON template and supported operations for `--patch-file`. This is the recommended first step before generating a patch file. No draft read or write is performed |
| `--inspect` | No | Inspect the draft without modifying it. Returns the draft projection including `attachments_summary` (with `part_id`, `cid`, `filename` for each attachment) and `inline_summary`. Use this to discover `part_id` or `cid` values before running `remove_attachment` or `remove_inline` |
| `--format <mode>` | No | Output format: `json` (default) / `pretty` / `table` / `ndjson` / `csv` |
| `--dry-run` | No | Print the request without executing it |

Note: Direct flags cover only common final-state edits. Use `--patch-file` for anything more granular.

## `--patch-file` Format

Recommended workflow for models:

1. Run `--print-patch-template`
2. Inspect the returned JSON structure
3. Generate a patch file that follows that structure
4. Run `--patch-file`

If you are not sure how to write the patch JSON, start with:

```bash
larksuite-cli mail +draft-edit --print-patch-template
```

`--patch-file` accepts a project-specific typed patch JSON format, not RFC 6902 JSON Patch.

Top-level structure:

```json
{
  "ops": [
    { "op": "set_subject", "value": "Updated subject" }
  ],
  "options": {
    "rewrite_entire_draft": false,
    "allow_protected_header_edits": false
  }
}
```

`options` fields:

- `rewrite_entire_draft`: Default `false`. Set to `true` only when the edit must synthesize or restructure body parts, for example adding a missing primary body part. Leave it `false` for normal subject, recipient, body, attachment, and inline-image edits.
- `allow_protected_header_edits`: Default `false`. Set to `true` only when the user explicitly wants to edit protected headers and understands the threading or delivery risk. Keep it `false` for normal usage.

### Subject & Body

`set_subject`

```json
{ "op": "set_subject", "value": "Updated subject" }
```

`set_body`

```json
{ "op": "set_body", "value": "new body" }
```

For coupled plain+HTML drafts, see [Body edit: coupled plain+HTML drafts](#body-edit-coupled-plainhtml-drafts).

### Recipients

`set_recipients`

```json
{ "op": "set_recipients", "field": "to", "addresses": [{ "address": "alice@example.com", "name": "Alice" }] }
```

`add_recipient`

```json
{ "op": "add_recipient", "field": "cc", "address": "alice@example.com", "name": "Alice" }
```

`remove_recipient`

```json
{ "op": "remove_recipient", "field": "cc", "address": "alice@example.com" }
```

### Headers

`set_header`

```json
{ "op": "set_header", "name": "X-Custom", "value": "abc" }
```

`remove_header`

```json
{ "op": "remove_header", "name": "X-Custom" }
```

### Attachments & Inline

**How to discover `part_id` / `cid`:** `remove_attachment`, `remove_inline`, and `replace_inline` require a `part_id` or `cid` to identify the target part. These values come from the draft's MIME structure and are **not** the same as the public API attachment IDs. To discover them, run `--inspect` first:

```bash
larksuite-cli mail +draft-edit --draft-id <draft_id> --inspect
```

The response `projection.attachments_summary` and `projection.inline_summary` list every part with its `part_id`, `cid`, `filename`, and `content_type`. Use these values in `remove_attachment` / `remove_inline` / `replace_inline` operations.

`add_attachment`

```json
{ "op": "add_attachment", "path": "./report.pdf" }
```

`remove_attachment`

```json
{ "op": "remove_attachment", "target": { "part_id": "1.3" } }
{ "op": "remove_attachment", "target": { "cid": "logo" } }
```

`target` accepts `part_id` or `cid`. Priority: `part_id` > `cid`.

`add_inline`

```json
{ "op": "add_inline", "path": "./logo.png", "cid": "logo" }
```

> **Critical: `add_inline` only adds the MIME binary part. It does NOT insert an `<img>` tag into the HTML body.**
> If you want the image to be visible in the email body, you **must** also use `set_body` to update the HTML body with the `<img src="cid:...">` tag. See [Insert an inline image into the body](#insert-an-inline-image-into-the-body) for the correct workflow.
> If you forget to add the `<img>` reference, the inline part becomes an orphaned attachment when sent.

`replace_inline`

```json
{ "op": "replace_inline", "target": { "part_id": "1.2" }, "path": "./new-logo.png", "filename": "new-logo.png", "content_type": "image/png" }
{ "op": "replace_inline", "target": { "cid": "logo" }, "path": "./new-logo.png" }
```

`filename` and `content_type` are optional in `replace_inline`. If omitted, the command keeps the original inline part's filename and content type. `target` accepts `part_id` or `cid`.

`remove_inline`

```json
{ "op": "remove_inline", "target": { "part_id": "1.2" } }
{ "op": "remove_inline", "target": { "cid": "logo" } }
```

Notes:

- `ops` are executed in order
- `target` accepts `part_id` or `cid`; priority: `part_id` > `cid`
- **`set_body` is a full replacement** — it replaces the entire body content, not an incremental edit. When the draft already has content (text, inline image references, etc.), you must preserve it. Never fabricate a new body from scratch; always start from the current body obtained via `--inspect` (see `body_html_summary`) or `drafts.get`, then apply your changes to that content.

## Return Value

On success:

```json
{
  "ok": true,
  "data": {
    "draft_id": "draft-id",
    "warning": "This edit flow has no optimistic locking. If the same draft is changed concurrently, the last writer wins."
  }
}
```

## Typical usage

### Fetch draft → edit → send

```bash
# 1. Fetch the draft to inspect current state
larksuite-cli mail user_mailbox.drafts get --params '{"user_mailbox_id":"me","draft_id":"<draft_id>"}'

# 2. Edit the draft
larksuite-cli mail +draft-edit --draft-id <draft_id> --set-subject 'Final version' --set-body '<p>Updated content</p>'

# 3. Send the draft
larksuite-cli mail user_mailbox.drafts send --params '{"user_mailbox_id":"me","draft_id":"<draft_id>"}'
```

### Remove an attachment from a draft

```bash
# 1. Inspect the draft to discover attachment part_id / cid values
larksuite-cli mail +draft-edit --draft-id <draft_id> --inspect
# Response includes projection.attachments_summary, e.g.:
#   [{"part_id":"1.3","filename":"report.pdf","content_type":"application/pdf"}]

# 2. Write a patch file targeting the part_id from step 1
cat > /tmp/patch.json << 'EOF'
{
  "ops": [
    { "op": "remove_attachment", "target": { "part_id": "1.3" } }
  ],
  "options": {}
}
EOF

# 3. Apply
larksuite-cli mail +draft-edit --draft-id <draft_id> --patch-file /tmp/patch.json
```

### Insert an inline image into the body

Adding an inline image requires **two coordinated edits**: (1) add the MIME part via `add_inline`, and (2) insert the `<img src="cid:...">` tag into the **existing** HTML body via `set_body`. You must preserve the original body content — never fabricate a new body from scratch.

```bash
# 1. Inspect the draft to get the current HTML body and discover existing inline parts
larksuite-cli mail +draft-edit --draft-id <draft_id> --inspect
# Response includes:
#   projection.body_html_summary: "<div>Original content<img src=\"cid:existing.png\" /></div>"
#   projection.inline_summary: [{"part_id":"1.1.2","cid":"existing.png", ...}]

# 2. Write a patch that:
#    - Uses set_body with the ORIGINAL body content, adding only the new <img> tag
#    - Uses add_inline to add the image binary as a MIME part
#    IMPORTANT: The set_body value must be based on the ORIGINAL body from step 1,
#    with the new <img> tag inserted. Do NOT discard the original content.
cat > /tmp/patch.json << 'EOF'
{
  "ops": [
    { "op": "set_body", "value": "<div>Original content<img src=\"cid:existing.png\" /><img src=\"cid:new-image\" /></div>" },
    { "op": "add_inline", "path": "./new-image.png", "cid": "new-image" }
  ],
  "options": {}
}
EOF

# 3. Apply
larksuite-cli mail +draft-edit --draft-id <draft_id> --patch-file /tmp/patch.json
```

**Common mistake:** Using `set_body` with a brand-new HTML body that does not include the original content. This destroys existing text and orphans existing inline images (`cid` references are lost), causing them to appear as unexpected attachments when the email is sent.

### Advanced edit with patch-file

```bash
# 1. Check the patch template
larksuite-cli mail +draft-edit --print-patch-template

# 2. Write a patch file (e.g., add a CC and remove an attachment)
cat > /tmp/patch.json << 'EOF'
{
  "ops": [
    { "op": "add_recipient", "field": "cc", "address": "carol@example.com", "name": "Carol" },
    { "op": "remove_attachment", "target": { "part_id": "1.3" } }
  ],
  "options": {}
}
EOF

# 3. Apply
larksuite-cli mail +draft-edit --draft-id <draft_id> --patch-file /tmp/patch.json
```

## Related Commands

- `larksuite-cli mail +draft-create` — create a new draft
- `larksuite-cli mail user_mailbox.drafts get` — fetch the raw draft content
- `larksuite-cli mail user_mailbox.drafts send` — send an existing draft
