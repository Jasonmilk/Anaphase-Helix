
# docs +fetch（获取飞书云文档）


## 命令

```bash
# 获取文档（默认 XML 格式，简洁模式）
lark-cli docs +fetch --api-version v2 --doc "https://xxx.feishu.cn/docx/Z1FjxxxxxxxxxxxxxxxxxxxtnAc"

# 直接传 token
lark-cli docs +fetch --api-version v2 --doc Z1FjxxxxxxxxxxxxxxxxxxxtnAc

# 获取 Markdown 格式
lark-cli docs +fetch --api-version v2 --doc Z1FjxxxxxxxxxxxxxxxxxxxtnAc --doc-format markdown

# 带 block ID（用于后续 block 级更新）
lark-cli docs +fetch --api-version v2 --doc Z1FjxxxxxxxxxxxxxxxxxxxtnAc --detail with-ids

# 全量导出（block ID + 样式 + 引用数据，用于编辑）
lark-cli docs +fetch --api-version v2 --doc Z1FjxxxxxxxxxxxxxxxxxxxtnAc --detail full

# 人类可读输出
lark-cli docs +fetch --api-version v2 --doc Z1FjxxxxxxxxxxxxxxxxxxxtnAc --format pretty
```

## 意图引导：选择正确的 `--detail` 级别

| 意图 | `--detail` | 说明 |
|------|-----------|------|
| **只读**：浏览或总结文档内容 | `simple`（默认） | 简洁 XML/Markdown，不含 block ID、样式属性、引用元数据 |
| **定位**：需要 block ID 与其他业务交互 | `with-ids` | 包含 block ID（如 `<p id="blkcnXXXX">`），可用于 `+update` 的 `--block-id` |
| **编辑**：任何修改文档内容的需求 | `full` | 包含 block ID + 样式属性 + 引用元数据，提供完整文档结构信息 |

**决策规则**：
- 如果用户只是想看/读/总结文档 → `simple`
- 如果后续需要用 `block_insert_after`/`block_replace`/`block_delete` → `with-ids` 或 `full`
- 如果涉及复杂编辑（保留样式、处理引用）→ `full`

## 返回值

```json
{
  "ok": true,
  "identity": "user",
  "data": {
    "document": {
      "document_id": "doxcnXXXX",
      "revision_id": 12,
      "content": "<title>标题</title><p>文档内容...</p>"
    }
  }
}
```

- **`document_id`**：文档标识符
- **`revision_id`**：文档版本号
- **`content`**：文档内容（格式由 `--doc-format` 决定）

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `--api-version` | 是 | 固定传 `v2` |
| `--doc` | 是 | 文档 URL 或 token（支持 `/docx/` 和 `/wiki/` 链接，自动提取 token） |
| `--doc-format` | 否 | 内容格式：`xml`（默认）\| `markdown` \| `text` |
| `--detail` | 否 | 导出详细程度：`simple`（默认）\| `with-ids` \| `full` |
| `--revision-id` | 否 | 文档版本号，-1 = 最新（默认 `-1`） |
| `--format` | 否 | 输出格式：json（默认）\| pretty（直接输出 content） |

### doc-format 格式说明

| 格式 | 说明 | 适用场景 |
|------|------|----------|
| `xml` | DocxXML 格式（默认） | 精确的块级结构，适合编辑和分析 |
| `markdown` | Lark-flavored Markdown | 可读性好，适合总结和展示 |

### detail 导出级别

| 级别 | export_block_id | export_style_attrs | export_cite_extra_data | 典型用途 |
|------|:-:|:-:|:-:|------|
| `simple` | ✗ | ✗ | ✗ | 只读浏览、内容总结（显式关闭所有导出选项） |
| `with-ids` | ✓ | - | - | 定位 block 后进行 block 级更新 |
| `full` | ✓ | ✓ | ✓ | 复杂编辑、保留完整格式 |

## 重要：图片、文件、画板的处理

**文档中的图片、文件、画板需要通过独立的 media shortcut 单独获取。**

### 识别格式

返回内容中，媒体文件以 XML 标签形式出现：

```xml
<!-- 图片 -->
<img token="Z1FjxxxxxxxxxxxxxxxxxxxtnAc" width="1833" height="2491"/>

<!-- 文件 -->
<source token="Z1FjxxxxxxxxxxxxxxxxxxxtnAc" name="skills.zip"/>

<!-- 画板 -->
<whiteboard token="Z1FjxxxxxxxxxxxxxxxxxxxtnAc"/>
```

### 获取步骤

1. 从 HTML 标签中提取 `token` 属性值
2. 如果目标是图片/文件素材，且用户只是想查看/预览，调用 [`lark-doc-media-preview`](lark-doc-media-preview.md)（`docs +media-preview`）：
   ```bash
   lark-cli docs +media-preview --token "提取的token" --output ./preview_media
   ```
3. 如果用户明确要下载，或目标是 `<whiteboard token="..."/>`，调用 [`lark-doc-media-download`](lark-doc-media-download.md)（`docs +media-download`）：
   ```bash
   lark-cli docs +media-download --token "提取的token" --output ./downloaded_media
   ```

## 重要：嵌入电子表格 / 多维表格的处理

返回内容中可能包含 `<sheet>`、`<bitable>`、`<cite file-type="sheets|bitable">` 等嵌入引用标签。这些标签的内部数据无法通过 `docs +fetch` 获取——提取标签中的 `token` 等关键属性后，切到 [`lark-sheets`](../../lark-sheets/SKILL.md) 或 [`lark-base`](../../lark-base/SKILL.md) 下钻读取。详见 [SKILL.md 快速决策](../SKILL.md) 中的路由表。

## 工具组合

| 需求 | 工具 |
|------|------|
| 获取文档文本 | `docs +fetch` |
| 预览图片/文件素材 | `docs +media-preview` |
| 下载图片/文件/画板 | `docs +media-download` |
| 创建新文档 | `docs +create` |
| 更新文档内容 | `docs +update` |

## 参考

- [lark-doc-create](lark-doc-create.md) — 创建文档（含完整 XML 语法参考）
- [lark-doc-update](lark-doc-update.md) — 更新文档
- [lark-doc-media-preview](lark-doc-media-preview.md) — 预览素材
- [lark-doc-media-download](lark-doc-media-download.md) — 下载素材/画板缩略图
