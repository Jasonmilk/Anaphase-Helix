# 转换路由：格式互转

处理 Office 文档与 PDF 的互转，以及 LaTeX 编译为 PDF。

---

## 路由分支

| 用户场景 | 分支 | 工具 |
|---------|------|------|
| 提供 .docx/.pptx/.xlsx 等，要求转为 PDF | **Office → PDF** | LibreOffice |
| 要求将 PDF 转为 Word/PPT/Excel | **PDF → Office** | LibreOffice |
| 提供 .tex 文件或要求 LaTeX 编译 | **LaTeX → PDF** | Tectonic + `compile_latex.py` |

---

## 分支 A：Office → PDF

### 依赖

需要 LibreOffice。检查是否已安装：

```bash
soffice --version
```

### 单文件转换

```bash
python3 scripts/pdf.py convert input.docx -o output.pdf
```

### 批量转换

```bash
soffice --headless --convert-to pdf --outdir ./output *.pptx *.docx *.xlsx
```

### 支持的格式

`.docx`、`.doc`、`.odt`、`.rtf`、`.pptx`、`.ppt`、`.odp`、`.xlsx`、`.xls`、`.ods`、`.csv`、`.txt`、`.html`

### 注意事项

- 转换保留原始排版、布局和字体（取决于系统字体可用性）
- 复杂的 Office 文档（如含大量图表的 PPT）转换可能有细微差异
- 中文字体缺失时可能显示为方块，需安装中文字体包

---

## 分支 B：PDF → Office

### 使用 LibreOffice 反向转换

```bash
# PDF 转 Word
soffice --headless --infilter="writer_pdf_import" --convert-to docx --outdir ./output input.pdf

# PDF 转 PPT（效果取决于 PDF 内容结构）
soffice --headless --convert-to pptx --outdir ./output input.pdf
```

### 局限性

- PDF → Office 的转换效果取决于 PDF 的内部结构
- 扫描型 PDF（纯图片）无法转换为可编辑文档
- 复杂排版（多栏、浮动图片）可能丢失格式
- 建议用户检查转换结果并手动调整

---

## 分支 C：LaTeX → PDF

### 第 1 步：安装 Tectonic

Tectonic 非预装环境。首先安装：

```bash
cd ~ && curl -fsSL https://drop-sh.fullyjustified.net | sh && ls -la tectonic
```

Tectonic 会安装到 `~/tectonic`（用户主目录）。

### 第 2 步：编译

**必须**使用 `compile_latex.py` 脚本编译。**禁止直接运行 tectonic。**

该脚本会自动：
- 过滤冗余的包下载日志
- 过滤编译进度信息
- 保留所有错误和警告
- 显示 PDF 统计信息（大小、页数、字数、图表数）

```bash
# 单次编译
python3 scripts/compile_latex.py main.tex

# 多次编译（用于交叉引用和参考文献）
python3 scripts/compile_latex.py main.tex --runs 2

# 保留完整日志（用于调试）
python3 scripts/compile_latex.py main.tex --keep-logs
```

### 第 3 步：检查与修复

编译后**必须检查**：
- 如果有错误，**必须修复**后重新编译
- 如果有版式问题（overfull/underfull box），也应尽力修复
- 使用 `--runs 2` 确保交叉引用和目录正确

### LaTeX 编写规范

#### 文档结构

```latex
\documentclass[12pt,a4paper]{article}
\usepackage[UTF8]{ctex}           % 中文支持
\usepackage{geometry}
\geometry{left=2.8cm, right=2.8cm, top=2.8cm, bottom=2.5cm}
\usepackage{hyperref}             % hyperref 必须最后加载
\hypersetup{colorlinks=true, linkcolor=blue, citecolor=blue}

\begin{document}
% 内容
\end{document}
```

#### 关键规则

- `hyperref` 包**必须最后加载**（在所有其他包之后）
- 长文档使用 `\input{}` 拆分为多个 .tex 文件
- 目录和引用必须可点击（通过 hyperref 实现）
- 中文文档必须使用 `ctex` 包

---

## 脚本参考

| 脚本 | 用途 |
|------|------|
| `pdf.py convert` | Office → PDF（调用 LibreOffice） |
| `compile_latex.py` | LaTeX → PDF（调用 Tectonic） |
| `pdf.sh latex` | LaTeX 编译的 Shell 入口 |
