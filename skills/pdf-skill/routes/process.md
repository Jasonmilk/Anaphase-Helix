# 处理路由：操作已有 PDF

使用 Python CLI 工具（pikepdf + pdfplumber）处理已有 PDF 文件。

---

## 第 0 步：检查并安装依赖（优先执行）

**使用 pdf.py 命令前先运行**——安装包需要时间。

```bash
scripts/setup.sh
```

该脚本仅检查状态，不自动安装。如有缺失，手动安装：
- Python 3: `apt install python3`
- 依赖: `pip install pikepdf pdfplumber --user`

---

## 命令参考

```
python3 scripts/pdf.py <命令> <子命令> [选项]
```

| 命令 | 说明 |
|------|------|
| `form info <pdf>` | 查看表单字段 |
| `form fill <pdf> -o <输出> -d <json>` | 填充表单字段 |
| `extract text <pdf> [-p 页码]` | 提取文本 |
| `extract table <pdf> [-p 页码]` | 提取表格 |
| `extract image <pdf> -o <目录>` | 提取图片 |
| `pages merge <pdf>... -o <输出>` | 合并 PDF |
| `pages split <pdf> -o <目录>` | 拆分为单页 |
| `pages rotate <pdf> <90\|180\|270> -o <输出>` | 旋转页面 |
| `pages crop <pdf> <左,下,右,上> -o <输出>` | 裁剪页面 |
| `meta get <pdf>` | 读取元数据 |
| `meta set <pdf> -o <输出> -d <json>` | 设置元数据 |

## 输出格式

所有命令输出 JSON：

```json
// 成功
{"status": "success", "data": {...}}

// 错误（输出到 stderr）
{"status": "error", "error": "错误类型", "message": "描述", "hint": "建议"}
```

## 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 成功 |
| 1 | 参数错误 |
| 2 | 文件未找到 |
| 3 | PDF 解析错误 |
| 4 | 操作失败 |

---

## 表单填充工作流

**第 1 步：查看表单字段**

```bash
python3 scripts/pdf.py form info input.pdf
```

输出示例：

```json
{
  "status": "success",
  "data": {
    "has_fields": true,
    "count": 5,
    "fields": [
      {"id": "name", "type": "text", "page": 1},
      {"id": "agree", "type": "checkbox", "states": ["/Yes", "/Off"], "checked_value": "/Yes", "page": 1},
      {"id": "country", "type": "dropdown", "options": [{"value": "US", "label": "US"}, {"value": "CN", "label": "CN"}], "page": 1}
    ]
  }
}
```

**第 2 步：填充表单**

```bash
python3 scripts/pdf.py form fill input.pdf -o output.pdf -d '{"name": "张三", "agree": "true", "country": "CN"}'
```

### 字段值规则

| 字段类型 | 值格式 | 示例 |
|---------|--------|------|
| text | 任意字符串 | `"name": "张三"` |
| checkbox | `"true"` 或 `"false"` | `"agree": "true"` |
| radio | options 中的选项值 | `"gender": "/Choice1"` |
| dropdown | options 中的选项值 | `"country": "CN"` |

**重要**：checkbox 字段使用 `"true"` 或 `"false"` 字符串值。脚本会自动转换为正确的 PDF 值（`/Yes`、`/On`、`/Off` 等）。

---

## 读取 PDF 内容

当用户要求"读取/阅读 PDF"、"帮我看看这个 PDF"、"总结 PDF 内容"时，使用 `extract text` 命令提取文本。

**推荐工作流：**

```bash
# 第 1 步：提取全文
python3 scripts/pdf.py extract text document.pdf

# 如果文档较长（>20 页），分页提取避免输出截断
python3 scripts/pdf.py extract text document.pdf -p 1-10
python3 scripts/pdf.py extract text document.pdf -p 11-20
```

**检查返回结果中的 `likely_scanned` 和 `warning` 字段：**

| 返回字段 | 含义 | 处理方式 |
|---------|------|---------|
| 无 `warning` | 正常提取 | 直接使用提取的文本 |
| `likely_scanned: true` + `total_chars: 0` | 纯扫描/图片 PDF | **走图片路径**：逐页导出为图片 → base64 → 传给模型视觉能力处理 |
| `likely_scanned: true` + 少量文本 | 部分扫描或图片为主 | 返回已提取的文本，同时对文本稀少的页走图片路径补充 |

**纯图片 PDF 处理流程：**

```bash
# 1. 导出每页图片
python3 scripts/pdf.py extract image document.pdf -o ./page_images/

# 2. 将图片转为 base64（Python 示例）
import base64, pathlib
for img in sorted(pathlib.Path("./page_images").iterdir()):
    b64 = base64.b64encode(img.read_bytes()).decode()
    # 3. 将 b64 作为图片消息传给模型，由模型视觉能力识别内容
```

逐页 base64 传给模型后，模型即可识别文字、表格、图表等内容，无需外部 OCR 工具。

---

## 文本和表格提取

**提取文本**：

```bash
python3 scripts/pdf.py extract text document.pdf
python3 scripts/pdf.py extract text document.pdf -p 1-3    # 仅第 1-3 页
python3 scripts/pdf.py extract text document.pdf -p 1,3,5  # 指定页码
```

**提取表格**：

```bash
python3 scripts/pdf.py extract table document.pdf
```

输出包含结构化表格数据：

```json
{
  "total_pages": 10,
  "extracted_pages": 10,
  "total_tables": 3,
  "tables": [
    {
      "page": 1,
      "table_index": 0,
      "rows": 5,
      "cols": 3,
      "data": [["表头1", "表头2", "表头3"], ["A", "B", "C"]]
    }
  ]
}
```

---

## 页面操作

**合并 PDF**：

```bash
python3 scripts/pdf.py pages merge a.pdf b.pdf c.pdf -o merged.pdf
```

**拆分 PDF**：

```bash
python3 scripts/pdf.py pages split document.pdf -o ./output_dir/
```

**旋转页面**：

```bash
python3 scripts/pdf.py pages rotate document.pdf 90 -o rotated.pdf
python3 scripts/pdf.py pages rotate document.pdf 180 -o rotated.pdf -p 1-3  # 指定页码
```

**裁剪页面**：

```bash
python3 scripts/pdf.py pages crop document.pdf 50,50,550,750 -o cropped.pdf
```

框格式：`左,下,右,上`，单位为点（1 英寸 = 72 点）。

---

## 元数据操作

**读取元数据**：

```bash
python3 scripts/pdf.py meta get document.pdf
```

**设置元数据**：

```bash
python3 scripts/pdf.py meta set document.pdf -o output.pdf -d '{"Title": "我的文档", "Author": "张三"}'
```

支持的字段：`Title`、`Author`、`Subject`、`Keywords`、`Creator`、`Producer`

---

## 脚本参考

| 脚本 | 用途 |
|------|------|
| `pdf.py` | 统一 CLI 入口 |
| `cmd_form.py` | 表单查看和填充 |
| `cmd_extract.py` | 文本、表格、图片提取 |
| `cmd_pages.py` | 合并、拆分、旋转、裁剪 |
| `cmd_meta.py` | 元数据读写 |

## 技术栈

| 库 | 用途 | 许可证 |
|----|------|--------|
| pikepdf | 表单填充、页面操作、元数据 | MPL-2.0 |
| pdfplumber | 文本和表格提取 | MIT |

---

## 重要注意事项

### 加密 PDF

**不支持。** CLI 命令不支持加密的 PDF。如果用户提供了加密 PDF，告知此功能不可用，建议用户先用其他工具解密。

### 大文件处理

| 文件大小 | 预期行为 |
|---------|---------|
| < 50 MB | 正常处理 |
| 50-200 MB | 可能较慢，1-2 分钟 |
| > 200 MB | 建议先拆分，或增加超时时间 |

**内存使用**：大约为文件大小的 2-3 倍。100MB 的 PDF 约需 300MB 内存。

### 扫描件 / 图片 PDF

如果 `extract text` 返回 `likely_scanned: true`，说明 PDF 内容为图片而非文本。此时使用 `extract image` 逐页导出图片，将每页转为 base64 后传给模型视觉能力进行内容识别，而非要求用户自行 OCR。

### 错误恢复

如果命令中途失败：
- **合并**：可能存在不完整的输出文件，删除后重试
- **拆分**：部分页面可能已写入，检查输出目录
- **表单填充**：原始文件不变（写入新文件）
