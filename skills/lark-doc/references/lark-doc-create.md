# docs +create（创建飞书云文档）

> **前置条件（MUST READ）：** 生成文档内容前，必须先用 Read 工具读取以下文件，缺一不可：
> 2. [`lark-doc-xml.md`](lark-doc-xml.md) — XML 语法规则（使用 Markdown 格式时改读 [`lark-doc-md.md`](lark-doc-md.md)）
> 3. [`lark-doc-style.md`](style/lark-doc-style.md) — 排版指南（元素选择、丰富度规则、颜色语义）
> 4. [`lark-doc-create-workflow.md`](style/lark-doc-create-workflow.md) — 从零创作工作流（Code-Act Loop、并行执行策略）
>
> **未读完以上文件就生成内容会导致格式错误或样式不达标。**

从 XML（默认）或 Markdown 内容创建一个新的飞书云文档。

> **⚠️ 格式选择规则：始终使用 XML 格式（默认），除非用户明确要求使用 Markdown。** XML 表达能力更强、支持更多 block 类型（callout、grid、checkbox 等），是推荐的首选格式。不要因为 Markdown 写起来更简单就自行切换为 Markdown。

## 命令

```bash
# 创建 XML 文档（默认格式，推荐）
lark-cli docs +create --api-version v2 --content '<title>项目计划</title><h1>目标</h1><ul><li>目标 1</li><li>目标 2</li></ul>'

# 创建到指定文件夹（XML）
lark-cli docs +create --api-version v2 --parent-token fldcnXXXX --content '<title>标题</title><p>首段内容</p>'

# 创建到个人知识库（XML）
lark-cli docs +create --api-version v2 --parent-position my_library --content '<title>标题</title><p>内容</p>'

# 仅当用户明确要求时才使用 Markdown
lark-cli docs +create --api-version v2 --doc-format markdown --content "# 项目计划\n\n## 目标\n\n- 目标 1\n- 目标 2"
```

## 返回值

```json
{
  "ok": true,
  "identity": "user",
  "data": {
    "document": {
      "document_id": "doxcnXXXXXXXXXXXXXXXXXXX",
      "revision_id": 1,
      "url": "https://xxx.feishu.cn/docx/doxcnXXXXXXXXXXXXXXXXXXX",
      "newblocks": [
        { "block_id": "blkcnXXXX", "block_type": "whiteboard", "token": "boardXXXX" }
      ]
    }
  }
}
```

- **`document.newblocks`**：本次操作新增的 block 列表（如画板），可从中提取 `token` 用于后续编辑


## 参数

| 参数                  | 必填 | 说明                                          |
| ------------------- | -- |---------------------------------------------|
| `--api-version`     | 是  | 固定传 `v2`                                    |
| `--content`         | 是  | 文档内容（XML 或 Markdown 格式）                     |
| `--doc-format`      | 否  | 内容格式：`xml`（默认，始终优先使用）\| `markdown`（仅用户明确要求时） |
| `--parent-token`    | 否  | 父文件夹或知识库节点 token（与 `--parent-position` 互斥）  |
| `--parent-position` | 否  | 父节点位置，如 `my_library`（与 `--parent-token` 互斥） |

## 最佳实践

- 文档标题从内容中自动提取（XML `<title>` 或 Markdown `#`），不要在内容开头重复写标题
- 创建较长的文档时，先创建基础内容，再用 `docs +update --command block_insert_after` 分段追加
- **视觉丰富度**：必须遵循 [`lark-doc-style.md`](style/lark-doc-style.md) 中的样式指南，主动使用结构化 block 丰富文档

## 参考

- [`lark-doc-create-workflow.md`](style/lark-doc-create-workflow.md) — 从零创作工作流（Code-Act Loop、并行执行策略）
- [`lark-doc-style.md`](style/lark-doc-style.md) — 文档样式指南（元素选择 + 丰富度规则 + 颜色语义）
- [`lark-doc-xml.md`](lark-doc-xml.md) — XML 语法规范
- [`lark-doc-fetch.md`](lark-doc-fetch.md) — 获取文档
- [`lark-doc-update.md`](lark-doc-update.md) — 更新文档
- [`lark-doc-media-insert.md`](lark-doc-media-insert.md) — 插入图片/文件到文档

