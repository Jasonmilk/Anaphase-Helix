# base +workspace-list-bases

> **前置条件：** 先阅读 [`../lark-shared/SKILL.md`](../../lark-shared/SKILL.md) 了解认证、全局参数和安全规则。

列出一个 workspace 下的 Base，支持分页。

## 推荐命令

```bash
larksuite-cli base +workspace-list-bases \
  --workspace-token wrk_xxx

larksuite-cli base +workspace-list-bases \
  --workspace-token wrk_xxx \
  --page-size 20 \
  --page-token xxx
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--workspace-token <token>` | 是 | Workspace Token |
| `--page-size <n>` | 否 | 每页数量（最大 100） |
| `--page-token <token>` | 否 | 分页标记 |
| `--format <fmt>` | 否 | 输出格式：json / pretty / table / csv / ndjson |
| `--dry-run` | 否 | 预览 API 调用，不执行 |

## API 入参详情

**HTTP 方法和路径：**

```
GET /open-apis/base/v3/workspaces/:workspace_token/bases
```

## 坑点

- ⚠️ 这是 `+xxx-list` 命令，禁止并发调用；批量跑多个 list 请求时只能串行执行。

## 参考

- [lark-base-workspace.md](lark-base-workspace.md) — base / workspace 索引页
- [lark-base-base-get.md](lark-base-base-get.md) — 读取单个 Base 详情
