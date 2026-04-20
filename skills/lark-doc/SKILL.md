---
name: lark-doc
version: 2.0.0
description: "飞书云文档：创建和编辑飞书文档。默认使用 DocxXML 格式（也支持 Markdown）。创建文档、获取文档内容（支持 simple/with-ids/full 三种导出详细度）、更新文档（八种指令：str_replace/str_delete/block_insert_after/block_replace/block_delete/block_move_after/overwrite/append）、上传和下载文档中的图片和文件、搜索云空间文档。当用户需要创建或编辑飞书文档、读取文档内容、在文档中插入图片、搜索云空间文档时使用；如果用户是想按名称或关键词先定位电子表格、报表等云空间对象，也优先使用本 skill 的 docs +search 做资源发现。"
metadata:
  requires:
    tools: ["lark_cli_exec"]
---

# docs (v2)

> **⚠️ API 版本：本 skill 使用 v2 API。所有 `docs +create`、`docs +fetch`、`docs +update` 命令必须携带 `--api-version v2`。**

```bash
# 常用示例
lark-cli docs +fetch  --api-version v2 --doc "文档URL或token"
lark-cli docs +create --api-version v2 --content '<title>标题</title><p>内容</p>'
lark-cli docs +update --api-version v2 --doc "文档URL或token" --command append --content '<p>内容</p>'
```

## 前置条件 — 生成内容前必读

**CRITICAL — 创建或编辑文档内容前，MUST 先用 Read 工具读取以下文件，缺一不可：**
2. [`lark-doc-xml.md`](references/lark-doc-xml.md) — XML 语法规则（仅当用户明确要求 Markdown 时改读 [`lark-doc-md.md`](references/lark-doc-md.md)）
3. 从零创建文档时必读 [`lark-doc-create-workflow.md`](references/style/lark-doc-create-workflow.md)；编辑已有文档时必读 [`lark-doc-update-workflow.md`](references/style/lark-doc-update-workflow.md)

**未读完以上文件就生成内容会导致格式错误或样式不达标。**

> **格式选择规则（全局）：** `docs +create` 和 `docs +update` 始终使用 XML 格式（`--doc-format xml`，即默认值），除非用户明确要求使用 Markdown。XML 支持 callout、grid、checkbox 等丰富 block 类型——不要因为 Markdown 更简单就自行切换。

## 画板编辑
> **不能直接编辑已有画板。**

### 创建画板
在 `docs +create` / `docs +update` 中用 `<whiteboard type="mermaid|plantuml">语法</whiteboard>` 直接创建带内容的画板。

### 高级用法：DSL 画板
Mermaid / PlantUML 无法满足时（架构图、对比图等）：
1. 用 `<whiteboard type="blank"></whiteboard>` 通过 `docs +update` 或 `docs +create` 创建空白画板
2. 从响应 `data.document.newblocks` 中提取画板 `token`
3. 参考 [`lark-whiteboard`](../lark-whiteboard/SKILL.md) skill 设计并上传 DSL

## 快速决策

- 用户需要在文档内**创建、复制或移动**资源块（画板、电子表格、多维表格等）时，必须先读取 [`lark-doc-xml.md`](references/lark-doc-xml.md) 的「三、资源块」章节
- 用户说"看一下文档里的图片/附件/素材""预览素材" → 用 `lark-cli docs +media-preview`
- 用户明确说"下载素材" → 用 `lark-cli docs +media-download`
- 如果目标是画板/whiteboard/画板缩略图 → 只能用 `lark-cli docs +media-download --type whiteboard`（不要用 `+media-preview`）
- 用户说"找一个表格""按名称搜电子表格""找报表""最近打开的表格" → 先用 `lark-cli docs +search` 做资源发现
- `docs +search` 不只搜文档/Wiki，结果里会直接返回 `SHEET` 等云空间对象
- 拿到 spreadsheet URL/token 后 → 切到 `lark-sheets` 做对象内部操作
- 用户说"给文档加评论""查看评论""回复评论""给评论加/删除表情 reaction" → 切到 `lark-drive` 处理
- 文档内容中出现嵌入的 `<sheet>`、`<bitable>` 或 `<cite file-type="sheets|bitable">` 标签时 → **必须主动提取 token 并切到对应技能下钻读取内部数据**，不能只呈现标签本身

| 标签 / 属性 | 提取字段 | 切到技能 |
|-|-|-|
| `<sheet token="..." sheet-id="...">` | `token` -> spreadsheet_token, `sheet-id` | [`lark-sheets`](../lark-sheets/SKILL.md) |
| `<bitable token="..." table-id="...">` | `token` -> app_token, `table-id` | [`lark-base`](../lark-base/SKILL.md) |
| `<cite type="doc" file-type="sheets" token="..." sheet-id="...">` | 同 `<sheet>` | [`lark-sheets`](../lark-sheets/SKILL.md) |
| `<cite type="doc" file-type="bitable" token="..." table-id="...">` | 同 `<bitable>` | [`lark-base`](../lark-base/SKILL.md) |
| `<synced_reference src-token="..." src-block-id="...">` | `src-token` -> doc_token, `src-block-id` -> block_id | 用 `docs +fetch` 读取 src-token 文档，定位 block |

**补充：** `docs +search` 也承担"先定位云空间对象，再切回对应业务 skill 操作"的资源发现入口角色；当用户口头说"表格/报表"时，也优先从这里开始。

## Shortcuts（推荐优先使用）

Shortcut 是对常用操作的高级封装（`lark-cli docs +<verb> [flags]`）。有 Shortcut 的操作优先使用。
