
# drive +export

把 `doc` / `docx` / `sheet` / `bitable` 导出为目标格式文件。这个 shortcut 内置有限轮询：

- 如果导出任务在轮询窗口内完成，会直接通过 ExecuteDownload 流式返回
- 如果轮询结束仍未完成，会返回 `ticket`、`ready=false`、`timed_out=true` 和 `next_command`
- 后续继续查结果时，改用 `drive +task_result --scenario export`
- `+task_result` 返回 `ready=true` 后，会自动完成下载

## 命令

```bash
# 导出新版文档为 pdf
lark-cli drive +export \
  --token "<DOCX_TOKEN>" \
  --doc-type docx \
  --file-extension pdf

# 导出旧版文档为 docx
lark-cli drive +export \
  --token "<DOC_TOKEN>" \
  --doc-type doc \
  --file-extension docx

# 导出 docx 为 markdown
# 注意：markdown 只支持 docx，底层走 /open-apis/docs/v1/content
lark-cli drive +export \
  --token "<DOCX_TOKEN>" \
  --doc-type docx \
  --file-extension markdown

# 导出电子表格为 xlsx
lark-cli drive +export \
  --token "<SHEET_TOKEN>" \
  --doc-type sheet \
  --file-extension xlsx

# 导出电子表格或多维表格为 csv 时，必须传 sub_id
lark-cli drive +export \
  --token "<SHEET_OR_BITABLE_TOKEN>" \
  --doc-type "<sheet|bitable>" \
  --file-extension csv \
  --sub-id "<SUB_ID>"
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--token` | 是 | 源文档 token |
| `--doc-type` | 是 | 源文档类型：`doc` / `docx` / `sheet` / `bitable` |
| `--file-extension` | 是 | 导出格式：`docx` / `pdf` / `xlsx` / `csv` / `markdown` |
| `--sub-id` | 条件必填 | 当 `sheet` / `bitable` 导出为 `csv` 时必填 |

## 关键约束

- `markdown` 只支持 `docx`
- `sheet` / `bitable` 导出为 `csv` 时必须带 `--sub-id`
- shortcut 内部固定有限轮询：最多 10 次，每次间隔 5 秒
- 轮询超时不是失败；会返回 `ticket`、`timed_out=true` 和 `next_command`，供后续继续查询

## 推荐续跑方式

```bash
# 第一步：先尝试直接导出
lark-cli drive +export \
  --token "<DOCX_TOKEN>" \
  --doc-type docx \
  --file-extension pdf

# 如果返回 ready=false / timed_out=true，再继续查
lark-cli drive +task_result \
  --scenario export \
  --ticket "<TICKET>" \
  --file-token "<DOCX_TOKEN>"
# +task_result 返回 ready=true 后，会自动完成下载
```

## 参考

- [lark-drive](../SKILL.md) -- 云空间全部命令
