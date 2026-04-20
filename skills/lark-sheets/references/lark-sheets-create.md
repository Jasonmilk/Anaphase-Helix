
# sheets +create（创建表格）


本 skill 对应 shortcut：`lark-cli sheets +create`。

特性：

- 一步创建表格并返回 URL
- 可选 `--headers/--data` 在创建后自动写入到第一个工作表的 A1 开始

> [!CAUTION]
> 这是**写入操作** —— 执行前必须确认用户意图。可以先用 `--dry-run` 预览。

## 命令

```bash
# 最简单：只创建
lark-cli sheets +create --title "仓库管理营收报表"

# 创建并写入表头 + 初始数据
lark-cli sheets +create --title "仓库管理营收报表" \
  --headers '["仓库","统计月份","入库金额","出库金额","销售收入","毛利率"]' \
  --data '[["华东一仓","2026-03",125000,98000,168000,"41.7%"]]'

# 创建到指定文件夹（folder_token）
lark-cli sheets +create --title "测试表" --folder-token "fldbc_xxx"

# 仅预览参数（不发请求）
lark-cli sheets +create --title "测试表" --dry-run
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--title <title>` | 是 | 表格标题 |
| `--folder-token <token>` | 否 | 云空间文件夹 token（创建到指定目录） |
| `--headers <json>` | 否 | 一维数组 JSON（表头；写入到 A1） |
| `--data <json>` | 否 | 二维数组 JSON（初始数据；紧跟表头写入） |
| `--dry-run` | 否 | 仅打印参数，不执行请求 |

## 输出

JSON，包含：

- `spreadsheet_token`
- `title`
- `url`

## 参考

- [lark-sheets-write](lark-sheets-write.md) — 后续覆盖写入
- [lark-sheets-append](lark-sheets-append.md) — 后续追加写入
