# 布局

> [!IMPORTANT]
> **这是一套基于 Flexbox 思想设计的精简版 Auto-Layout DSL。**
> 它的底层引擎是 Yoga，所以它的**行为逻辑**基本等同于 CSS Flexbox，你可以利用已有的前端排版心智来构建页面。
> **注意差异**：`alignItems` 默认值为 `'start'`（CSS Flexbox 默认 `stretch`）。需要等高卡片时必须显式写 `alignItems: 'stretch'`。
> **但是（极度重要）**：DSL 的语法是严格受限的白名单！你**绝对不能**在 JSON 中直接写原生 CSS 属性（比如不支持 `alignSelf`, `flexWrap`, `margin` 等）。只能使用下面表格中列出的映射属性！

## DSL 与 CSS Flexbox 属性映射及限制

| DSL 属性 (你只能写这个) | 对应的 CSS 心智模型 (它的底层行为) | 严格限制 |
|-----------------------|-----------------------------------|--------|
| `layout: 'horizontal'` | `display: flex; flex-direction: row` | **不写 layout = 绝对定位（非 flex）！** |
| `layout: 'vertical'` | `display: flex; flex-direction: column` | 同上 |
| `layout: 'none'` | `position: absolute`（子节点用 x/y） | 子节点不能用 `fill-container`；`fit-content` 仍可用于含文字节点（引擎通过 Yoga measureFunc 测量） |
| `width/height: 'fill-container'` | `flex: 1`（主轴） / `align-self: stretch`（交叉轴） | **必须有祖先提供确定尺寸！** 引擎会自动处理拉伸，无需手写 alignSelf。 |
| `width/height: 'fit-content'` | `width/height: auto`（内容撑开，类似 max-content） | — |
| `alignItems` | 同 CSS `align-items` | 仅支持：`'start'`, `'center'`, `'end'`, `'stretch'`（注意：没有 `flex-` 前缀） |
| `justifyContent` | 同 CSS `justify-content` | 仅支持：`'start'`, `'center'`, `'end'`, `'space-between'`, `'space-around'` |
| `gap` | 同 CSS `gap` | **绝对必填**，即使 `layout: 'none'` 也必须写 `0`，不可省略！ |
| `padding` | 同 CSS `padding` | **绝对必填**，即使不需要也必须写 `0`，不可省略！支持 `number` / `[v,h]` / `[t,r,b,l]` |

## DSL 注意事项

1. **frame 必须写 layout 属性**，不写时子节点全堆在左上角。
2. **🚨 致命的 fill-container 死锁陷阱（极高频错误！）🚨**：使用 `fill-container` 时，祖先链中必须有固定宽度（或高度），否则和 `fit-content` 形成死锁，尺寸退化为 0。

   ❌ 错误：horizontal 父 width fit-content + 子 width fill-container = 死锁
   ```json
   { "type": "frame", "layout": "horizontal", "width": "fit-content", "children": [
     { "type": "rect", "width": "fill-container" }
   ]}
   ```
   ❌ 错误：vertical 父 height fit-content + 子 height fill-container = 同理死锁
   ```json
   { "type": "frame", "layout": "vertical", "height": "fit-content", "children": [
     { "type": "rect", "height": "fill-container" }
   ]}
   ```
   ✅ 正确：祖先在对应轴有固定尺寸
   ```json
   { "type": "frame", "layout": "horizontal", "width": 1200, "children": [
     { "type": "rect", "width": "fill-container" }
   ]}
   ```
3. **含文字节点高度用 fit-content**，引擎不支持 overflow，写死高度会截断文字。
4. **Shape 节点有内边距**：rect/ellipse/diamond/triangle 各边 12px；cylinder 垂直 +42px。
5. **不支持 flex-wrap**，需要换行时用嵌套 frame 模拟。
6. **图层顺序**：数组中越靠后的节点层级越高。需要叠加标注时，放在数组最后。

---

## 怎么选择布局方式

先想清楚你要表达的信息之间是什么关系，再选排列方式：

| 你要表达的关系 | 怎么排 | DSL 写法 |
|-------------|-------|---------|
| 先后顺序、层级从上到下 | 纵向堆叠 | `layout: 'vertical'` |
| 并列、同等重要、可对比 | 横向等分 | `layout: 'horizontal'` + `alignItems: 'stretch'` + `width: 'fill-container'` |
| 区域有名称，名称在侧边 | 侧标签 + 内容并排 | 横向 frame: [text(标签), frame(内容)] |
| 多个大分区，各自独立 | 分区纵向排列 | 纵向 frame 包多个彩色 frame |
| 一行放不下，需要换行 | 嵌套横向 frame 模拟换行 | 纵向 frame 包多个横向 frame |
| 节点位置本身有含义（拓扑、地图） | 绝对定位 | `layout: 'none'` + x/y |

这些可以自由嵌套组合。比如：纵向堆叠(标题) + 分区纵向排列(多个层) + 每个层内横向等分(节点)。

---

## 布局示例

### 纵向堆叠（标题 + 内容）

告诉读者"这是什么"，再展示具体内容：

```json
{
  "type": "frame", "layout": "vertical", "gap": 28, "padding": 32,
  "width": 1200, "height": "fit-content",
  "children": [
    { "type": "text", "width": "fill-container", "height": "fit-content",
      "text": "图表标题", "fontSize": 24, "textAlign": "center" },
    ...内容...
  ]
}
```

### 横向等分（并列元素）

表达"这些东西是平级的，同等重要"：

```json
{
  "type": "frame", "layout": "horizontal", "gap": 16, "padding": 0,
  "width": "fill-container", "height": "fit-content",
  "alignItems": "stretch",
  "children": [
    { "type": "rect", "width": "fill-container", "height": "fit-content",
      "textAlign": "center", "verticalAlign": "middle", "text": "A" },
    { "type": "rect", "width": "fill-container", "height": "fit-content",
      "textAlign": "center", "verticalAlign": "middle", "text": "B" }
  ]
}
```

`alignItems: 'stretch'` + `width: 'fill-container'` = 等宽等高。

### 侧标签 + 内容

给一个区域取名字，读者知道这块是什么：

```json
{
  "type": "frame", "layout": "horizontal", "gap": 24, "padding": 0,
  "width": "fill-container", "height": "fit-content",
  "alignItems": "center",
  "children": [
    { "type": "text", "width": 160, "height": "fit-content",
      "text": "区域名称", "fontSize": 20, "textColor": "#1F2329", "textAlign": "right" },
    { "type": "frame", "width": "fill-container", "height": "fit-content",
      ...区域内容...
    }
  ]
}
```

不要用 frame 的 `title` 属性做标签——渲染为极小标题栏，不可读。

### 分区纵向排列

把内容划分为几个大区域，每个区域用不同颜色区分（颜色从 style 文件的色板选取）：

```json
{
  "type": "frame", "layout": "vertical", "gap": 28, "padding": 0,
  "width": "fill-container", "height": "fit-content",
  "children": [
    { "type": "frame", "borderRadius": 8,
      "layout": "horizontal", "gap": 16, "padding": 20, ...区域1... },
    { "type": "frame", "borderRadius": 8,
      "layout": "horizontal", "gap": 16, "padding": 20, ...区域2... }
  ]
}
```

### 模拟换行

一行放不下时，拆成多个横向 frame：

```json
{
  "type": "frame", "layout": "vertical", "gap": 8, "padding": 0,
  "children": [
    { "type": "frame", "layout": "horizontal", "gap": 8, "padding": 0,
      "children": [item1, item2, item3, item4] },
    { "type": "frame", "layout": "horizontal", "gap": 8, "padding": 0,
      "children": [item5, item6] }
  ]
}
```

---

## 绝对定位

当节点位置本身有含义（拓扑图、地图、时间线轴）时用绝对定位。

大多数图表优先用 Flex——自动排版，不会重叠。

### 混合布局

模块内部用 Flex 自动排版，模块之间用绝对定位自由摆放。每个模块是一个带 x/y 的 flex frame：

```json
{
  "type": "frame", "id": "module-a", "x": 100, "y": 100,
  "width": 300, "height": "fit-content",
  "layout": "vertical", "gap": 8, "padding": 16,
  "children": [
    { "type": "rect", "width": "fill-container", "height": "fit-content", "text": "内容1" },
    { "type": "rect", "width": "fill-container", "height": "fit-content", "text": "内容2" }
  ]
}
```

### 两阶段绘图

先出骨架图导出坐标，再基于坐标补充连线和注解：

```bash
npx -y @byted-ratio/whiteboard-cli -i skeleton.json -o step1.png -l coords.json
```

`coords.json` 包含每个带 id 节点的精确坐标（absX, absY, width, height）。

---

## 常用间距和尺寸

根据内容复杂度调整，以下是常用范围：

| 参数 | 常用范围 | 说明 |
|------|---------|------|
| 整图宽度 | 1000-1400px | — |
| 分区之间间距 | 24-32px | — |
| 同分区内节点间距 | 12-16px | — |
| 有连线的节点间距 | >= 40px | 给箭头留空间 |
| 分区内边距 | 16-24px | — |
| 侧标签宽度 | 120-180px | — |
