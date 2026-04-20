
# drive +upload

上传文件到飞书云空间。

## 命令

```bash
# 上传文件到指定文件夹
lark-cli drive +upload --url https://example.com/report.pdf --folder-token fldbc_xxx

# 自定义上传后的文件名
lark-cli drive +upload --url https://example.com/report.pdf --name "季度总结.pdf"

# 上传到根目录（省略 --folder-token）
lark-cli drive +upload --url https://example.com/data.xlsx

# 查看完整参数定义
lark-cli drive +upload --help
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--url <url>` | 是 | HTTPS URL，文件从该地址下载后上传到云空间（最大 20MB） |
| `--folder-token <token>` | 否 | 目标文件夹 token（默认：根目录） |
| `--name <name>` | 否 | 上传后的文件名（默认：从 URL 推导） |

## 输出

```json
{"file_token": "...", "file_name": "...", "size": 12345}
```

> [!CAUTION]
> 这是**写入操作** —— 执行前必须确认用户意图。

## 参考

- [lark-drive](../SKILL.md) -- 云空间全部命令
