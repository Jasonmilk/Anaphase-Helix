# base +base-copy


复制一个已有 Base；可选只复制结构，不复制内容。

## 推荐命令

```bash
lark-cli base +base-copy \
  --base-token app_xxx \
  --name "Copied Base"

lark-cli base +base-copy \
  --base-token app_xxx \
  --name "Copied Base" \
  --folder-token fld_xxx \
  --time-zone Asia/Shanghai \
  --without-content
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--base-token <token>` | 是 | 源 Base Token |
| `--name <name>` | 否 | 新 Base 名称 |
| `--folder-token <token>` | 否 | 目标文件夹 token |
| `--time-zone <tz>` | 否 | 时区，如 `Asia/Shanghai` |
| `--without-content` | 否 | 只复制结构，不复制内容 |

## API 入参详情

**HTTP 方法和路径：**

```
POST /open-apis/base/v3/bases/:base_token/copy
```

## 返回重点

- 返回 `base`。
- CLI 会额外标记 `copied: true`。
- 回复结果时，必须主动返回新 Base 的可访问链接：
  - 优先使用返回结果中的 `base.url`
  - 同时返回新 Base 的 token；字段名以实际返回为准，常见为 `base_token` 或 `app_token`
  - 如果本次返回没有 `url`，至少返回新 Base 的名称和 token
## 工作流

> [!CAUTION]
> 这是**写入操作** — 执行前必须向用户确认。

1. 先确认源 Base Token。
2. `--name`、`--folder-token`、`--time-zone` 都是可选项；用户没要求时不要为这些可选参数额外追问。
3. 只要结构时，显式传 `--without-content`。
4. 复制成功后，整理并返回：新 Base 名称、token，以及响应中已有的可访问链接。

## 参考

- [lark-base-workspace.md](lark-base-workspace.md) — base / workspace 索引页
- [lark-base-base-create.md](lark-base-base-create.md) — 创建全新 Base
