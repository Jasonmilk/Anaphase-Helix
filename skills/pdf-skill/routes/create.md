# 创建路由：HTML → PDF

使用 HTML + Playwright + Paged.js 从零创建专业 PDF。

**开始创建前，必须先读取 `design/design.md` 了解设计规范。**

---

## 第 0 步：安装依赖（优先执行，在后台运行）

**在写 HTML 之前立即在后台运行**——下载 Chromium 需要时间。

```bash
# 一键安装所有依赖（幂等，可重复运行）
scripts/pdf.sh fix
```

该命令依次执行：
1. `cd scripts && npm install` — 安装本地 playwright 依赖
2. `npm install -g playwright` — 全局安装 playwright
3. `npx playwright install chromium` — 下载 Chromium 浏览器
4. `pip install pikepdf pdfplumber` — 安装 Python 依赖

**在后台运行安装的同时，立即开始编写 HTML。**

### 如果 `pdf.sh fix` 失败或无输出

手动逐步安装：

```bash
cd /path/to/.skill/pdf-skill/scripts
npm install
npx playwright install chromium
```

### 如果 `html_to_pdf.js` 运行后无输出

这通常表示 playwright 未安装。运行：

```bash
cd /path/to/.skill/pdf-skill/scripts && npm install && npx playwright install chromium
```

然后重试。

---

## 第 1 步：编写 HTML

### 关键规则

1. **禁止加载 Paged.js**：转换脚本会自动注入；重复加载会导致页数翻倍和布局损坏
2. **禁止使用 CSS counter**：Paged.js 不兼容 CSS 计数器。使用 `data-*` 属性或手动编号
3. **禁止使用 JS 图表库**：ECharts、Chart.js、D3.js、Plotly 等动态渲染的 JS 库与 Paged.js 分页冲突
4. **禁止加载外部字体 CDN**：Google Fonts、Adobe Fonts 等外部字体服务在沙箱/离线环境中会被代理阻断（407 错误），导致页面渲染空白。**必须使用系统字体**

### 图表与公式

| 类型 | 方案 | 说明 |
|------|------|------|
| 流程图、时序图、架构图 | **Mermaid** | 渲染为静态 SVG，必须设 `theme:'neutral'` |
| 数据图表（柱/线/饼） | **`<img>` 标签** | 用 matplotlib 预生成图片再嵌入 |
| 数学公式 | **KaTeX** | 行内用 `\(...\)`，独立公式用 `\[...\]` |

**图表尺寸规则**：始终使用**横向**比例（宽 > 高），如 `figsize=(10, 6)`。禁止方形或竖向比例，防止溢出页面。

### Mermaid 使用

```html
<script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
<script>mermaid.initialize({startOnLoad: true, theme: 'neutral'});</script>

<div class="mermaid">
graph TD
    A[开始] --> B{判断}
    B -->|是| C[执行]
    B -->|否| D[结束]
</div>
```

### KaTeX 使用

```html
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex/dist/katex.min.css">
<script src="https://cdn.jsdelivr.net/npm/katex/dist/katex.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/katex/dist/contrib/auto-render.min.js"
        onload="renderMathInElement(document.body)"></script>
```

---

## 第 2 步：设计（参照 design/design.md）

### 页面基础设置

```css
@page {
    size: A4;
    margin: 2.8cm 2.8cm 2.5cm 2.8cm;
    @bottom-center {
        content: counter(page);
        font-size: 9pt;
        color: #666;
    }
}
```

### CSS 变量（统一设计 Token）

每份文档必须在 `:root` 中定义以下变量，确保全文一致性：

```css
:root {
    --accent: #2D5F8A;       /* 主色调，从配色推荐表选择 */
    --accent-lt: #E8F0F8;    /* 主色调浅色版，用于表格交替行 */
    --bg-cover: #0F1F2E;     /* 封面背景色 */
    --text-cover: #F0EDE6;   /* 封面文字色 */
    --font-display: Georgia, 'Times New Roman', 'SimSun', serif;  /* 封面/标题字体（系统字体） */
    --font-body: 'SimSun', 'Noto Serif CJK SC', 'Source Han Serif SC', Georgia, serif; /* 正文字体（系统字体） */
    --font-sans: 'Microsoft YaHei', 'Noto Sans CJK SC', 'Source Han Sans SC', Arial, sans-serif; /* 无衬线字体（系统字体） */
}
```

**禁止使用 `@import url('https://fonts.googleapis.com/...')`。** 外部字体在沙箱中不可用。

### 封面结构

封面必须单独一页，通过 `break-after: page` 与正文分离。

**设计风格封面骨架**（推荐大多数场景使用）：

```html
<div class="cover" style="
    background: var(--bg-cover);
    color: var(--text-cover);
    height: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 4cm;
    break-after: page;
">
    <h1 style="font-family: var(--font-display); font-size: 36pt; line-height: 1.2; margin-bottom: 1cm;">
        文档标题
    </h1>
    <div style="width: 60px; height: 4px; background: var(--accent); margin-bottom: 1cm;"></div>
    <p style="font-size: 14pt; opacity: 0.8;">作者 · 日期</p>
</div>
```

**极简风格封面骨架**（学术论文等）：

```html
<div class="cover" style="
    height: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    break-after: page;
">
    <h1 style="font-size: 24pt; margin-bottom: 2cm;">论文标题</h1>
    <p style="font-size: 14pt; color: #555;">作者姓名</p>
    <p style="font-size: 12pt; color: #777;">所属机构</p>
    <p style="font-size: 12pt; color: #777;">日期</p>
</div>
```

### 正文排版

#### 学术风格（默认）

默认输出应模拟 LaTeX 学术论文风格，而非网页/UI 风格。

**禁止的 UI 组件**：

| 禁止 | 替代方案 |
|------|---------|
| 卡片组件（带边框 + 头部） | 三线表或普通段落 |
| 统计仪表盘（数字卡片网格） | 用表格展示数据 |
| 深色标题栏 | 粗体标题 + 细边框或左边框 |
| 时间线组件 | 编号列表或表格 |
| 深色代码块 | 浅灰背景 `#f5f5f5` |
| 圆角边框 | 直角或无边框 |
| 阴影效果 | 无阴影 |

#### 颜色标准

| 元素 | 颜色规则 |
|------|---------|
| 正文文字 | `#1a1a1a`（接近纯黑） |
| 章节标题 | `#000` 或 `var(--accent)` |
| 表格边框 | `#ddd`（浅灰） |
| 表头背景 | `var(--accent)` + 白色文字 |
| 交替行 | `var(--accent-lt)` |
| 代码背景 | `#f5f5f5` |
| 引用/高亮框 | 左边框 `var(--accent)` 3px |

#### 三线表

学术文档中的表格应使用三线表样式：

```css
table {
    width: 100%;
    border-collapse: collapse;
    margin: 1em 0;
    font-size: 10pt;
}
th {
    border-top: 2px solid #000;
    border-bottom: 1px solid #000;
    padding: 8px 12px;
    text-align: left;
    background: var(--accent);
    color: white;
}
td {
    padding: 6px 12px;
    border-bottom: 1px solid #eee;
}
tr:last-child td {
    border-bottom: 2px solid #000;
}
```

#### 定理/定义框

```css
.theorem {
    border-left: 3px solid #333;
    padding-left: 1em;
    margin: 1em 0;
}
.theorem-title { font-weight: bold; }
.theorem-content { font-style: italic; }
```

#### 脚注

```css
.footnote {
    font-size: 8pt;
    color: #666;
    border-top: 1px solid #ccc;
    padding-top: 0.5em;
    margin-top: 2em;
}
```

### 页眉页脚（使用 Paged.js string-set）

```css
h1 { string-set: chapter-title content(text); }

@page {
    @top-left {
        content: string(chapter-title);
        font-size: 9pt;
        color: #999;
    }
    @bottom-center {
        content: counter(page);
        font-size: 9pt;
    }
}

@page:first {
    @top-left { content: none; }
    @bottom-center { content: none; }
}
```

---

## 第 3 步：转换为 PDF

```bash
node scripts/html_to_pdf.js document.html
node scripts/html_to_pdf.js document.html --output output.pdf
```

转换后脚本会输出：
- 页数、字数统计、图表数量
- **溢出检测**：如果 `pre`、`table`、`figure`、`img` 等超出页面宽度会发出警告
- **CSS Counter 检测**：如果使用了 CSS 计数器会发出警告（与 Paged.js 不兼容）
- 异常页检测（空白页、低内容页）

如果检测到溢出，为溢出元素添加 `max-width: 100%`。

---

## 重要注意事项

### CJK（中日韩）文字支持

中文文档必须指定中文字体栈，防止回退到英文字体导致方块字：

```css
body {
    font-family: var(--font-body);
    line-height: 1.8;
    text-align: justify;
}
```

### 长文档拆分

超过 20 页的文档，建议将内容拆分为多个 `<section>`，每个 `<section>` 对应一个章节。这有助于 Paged.js 正确分页。

### 图片处理

- 所有图片必须设置 `max-width: 100%` 防止溢出
- 图片标题使用 `<figcaption>` 标签
- 图片编号格式：`图 章号-序号`（如"图 3-1"）

```html
<figure id="fig-3-1">
    <img src="chart.png" style="max-width: 100%;">
    <figcaption>图 3-1：XXX分析结果</figcaption>
</figure>
```

### 参考文献

中文文档使用 GB/T 7714 格式：

```html
<section class="references">
    <h2>参考文献</h2>
    <ol>
        <li id="ref1">作者. 标题[J]. 期刊名, 年, 卷(期): 页码.</li>
        <li id="ref2">作者. 书名[M]. 出版地: 出版社, 年.</li>
    </ol>
</section>
```

---

## 故障排查

### `html_to_pdf.js` 运行后无任何输出

**原因**：Playwright 未安装到本地 `node_modules`。

**修复**：

```bash
cd /path/to/.skill/pdf-skill/scripts
npm install
npx playwright install chromium
```

然后重新运行 `node html_to_pdf.js document.html`。

### PDF 内容为空白

**可能原因**：
1. HTML 中使用了外部字体 CDN（`@import url('https://fonts.googleapis.com/...')`），被代理拦截 → **删除所有外部字体引用，使用系统字体**
2. HTML 中手动加载了 Paged.js → **删除，脚本会自动注入**
3. Paged.js 分页超时 → 检查 HTML 结构是否过于复杂

### Chromium 启动失败

**修复**：

```bash
npx playwright install-deps chromium   # 安装系统级依赖（需 root）
npx playwright install chromium         # 重新安装 Chromium
```

### 页面溢出或截断

在 HTML 中为所有图片和表格添加：

```css
img { max-width: 100%; height: auto; }
table { width: 100%; table-layout: fixed; word-wrap: break-word; }
pre { white-space: pre-wrap; word-wrap: break-word; overflow-x: hidden; }
```
