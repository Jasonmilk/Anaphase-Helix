---
name: lark-contact
version: 1.0.0
description: "飞书通讯录：查询组织架构、人员信息和搜索员工。获取当前用户或指定用户的详细信息、通过关键词搜索员工（姓名/邮箱/手机号）。当用户需要查看个人信息、查找同事 open_id 或联系方式、按姓名搜索员工、查询部门结构时使用。"
metadata:
  requires:
    tools: ["lark_cli_exec"]
---

# contact (v1)

## ID 获取指引

本技能是通过搜索获取 **用户 open_id**（`ou_xxx`）的主要入口。其他业务技能（如 lark-im、lark-calendar、lark-task、lark-vc）在需要通过搜索来获取用户 ID 时，应使用本技能的 `+search-user` 能力。

- **搜索用户获取 open_id**：`lark-cli contact +search-user --query "姓名或邮箱"`，从返回结果中提取 `open_id`。
- **搜索群聊获取 chat_id**：若需要通过搜索获取群聊 ID（`oc_xxx`），参考 [lark-im](../lark-im/SKILL.md) 技能，使用 `lark-cli im +chat-search` 按群名关键词搜索群聊。

## Shortcuts（推荐优先使用）

Shortcut 是对常用操作的高级封装（`lark-cli contact +<verb> [flags]`）。有 Shortcut 的操作优先使用。

| Shortcut | 说明 |
|----------|------|
| [`+search-user`](references/lark-contact-search-user.md) | Search users (results sorted by relevance) |
| [`+get-user`](references/lark-contact-get-user.md) | Get user info (omit user_id for self; provide user_id for specific user) |
