# base +base-create


创建一个新的 Base；可选指定父文件夹和时区。

## 推荐命令

```bash
lark-cli base +base-create \
  --name "New Base"

lark-cli base +base-create \
  --name "项目管理" \
  --folder-token fld_xxx \
  --time-zone Asia/Shanghai
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--name <name>` | 是 | 新 Base 名称 |
| `--folder-token <token>` | 否 | 目标文件夹 token |
| `--time-zone <tz>` | 否 | 时区，如 `Asia/Shanghai` |

## API 入参详情

**HTTP 方法和路径：**

```
POST /open-apis/base/v3/bases
```

## 返回重点

- 返回 `base`。
- CLI 会额外标记 `created: true`。
- 回复结果时，必须主动返回新 Base 的可访问链接：
  - 优先使用返回结果中的 `base.url`
  - 同时返回新 Base 的 token；字段名以实际返回为准，常见为 `base_token` 或 `app_token`
  - 如果本次返回没有 `url`，至少返回新 Base 的名称和 token
## 工作流

> [!CAUTION]
> 这是**写入操作** — 执行前必须向用户确认。

1. 先确认 Base 名称。
2. `--folder-token`、`--time-zone` 都是可选项；用户没要求时不要为此额外追问。
3. 创建成功后，整理并返回：Base 名称、token，以及响应中已有的可访问链接。

## 参考

- [lark-base-workspace.md](lark-base-workspace.md) — base / workspace 索引页
- [lark-base-base-copy.md](lark-base-base-copy.md) — 复制 Base
