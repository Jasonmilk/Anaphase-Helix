# 视觉风格

> [!IMPORTANT]
> **用户配色优先。** 如果用户在 prompt 中指定了配色方案、色值、主题色、品牌色或风格偏好（如"深色主题"、"蓝白配色"、"用 #1890FF"、"科技感"），
> 则以用户指定的配色为准，跳过下方"默认色板"章节，直接使用用户的颜色构建色板。
> 仍然必须遵守下方"结构规则"章节的所有规则（分组区分、层级对比、边框清晰、间距规范）。

---

## 结构规则

画图是为了把信息表达清楚。以下结构规则**无论使用什么配色都必须遵守**。

### 第一步：分组 — 用颜色告诉读者"哪些是一类的"

**必须为每个分组分配一种颜色。** 没有颜色区分的图读者无法快速识别分组。

选 2-4 种颜色，每种代表一个分组。同一分组的所有成员视觉完全一致。

**同级同样式**：逻辑上平级的节点（同组兄弟、同层成员），fillColor、borderColor、borderWidth、borderRadius、fontSize 不能有任何一个不同。读者一眼就能看出哪些节点是一伙的。

### 第二步：分层 — 用视觉重量告诉读者"先看什么、后看什么"

外层重、内层轻，读者的视线自然从整体结构走向具体细节：

- **外层（大分区）**：浅色填充做背景 — 建立视觉分区
- **中层（子分组标题）**：中等色填充 — 标识类别
- **内层（具体内容）**：最浅色或白色填充 — 最朴素，不抢注意力

### 第三步：保持清晰 — 让每个元素都能被清楚辨认

- **边框**：所有形状节点都有边框，边界清晰（borderWidth=2, borderColor 用同色系深色）
- **间距**：相邻元素之间有间距，不粘连不压盖。有连线的节点间 gap >= 40（给箭头留空间），其他情况 gap >= 8
- **文字**：够大够深，在背景上清晰可读（fontSize >= 14）。浅色背景用深色文字，深色背景用浅色文字
- **连线**：使用中性色（灰色或黑色），不和节点颜色抢注意力

### 统一参数

| 参数 | 值 | 为什么 |
|------|---|--------|
| borderWidth | 2 | 让边框清晰可见 |
| borderRadius | 8 | 统一的圆角，整洁 |
| gap（最小值） | 8 | 元素不粘连 |
| padding（最小值） | 8 | 内容不贴边 |
| gap（有连线时） | 40 | 给箭头留空间 |
| fontSize（正文） | >= 14 | 可读 |
| fontSize（标题） | >= 24 | 醒目 |
| fontSize（辅助） | >= 13 | 不费眼 |

---

## 默认色板

> **以下色板仅在用户未指定配色时使用。** 如果用户指定了配色，跳过本章节。

**每张图必须使用马卡龙色区分分组。** 纯黑白灰的图是不可接受的——颜色是帮助读者快速理解结构的重要工具。从下方色板中选 2-4 种颜色，主动为不同分组上色。

| 色名 | fillColor（浅色填充） | borderColor（深色边框） | textColor |
|------|---------------------|----------------------|-----------|
| 浅紫 | #EAE2FE | #8569CB | #1F2329 |
| 浅蓝 | #F0F4FC | #5178C6 | #1F2329 |
| 浅绿 | #DFF5E5 | #509863 | #1F2329 |
| 浅黄 | #FEF1CE | #D4B45B | #1F2329 |
| 浅红 | #FEE3E2 | #D25D5A | #1F2329 |

白色节点的边框色取决于它属于哪个分组：

```
属于蓝色分组: fillColor="#FFFFFF"  borderColor="#5178C6"  borderWidth=2
属于紫色分组: fillColor="#FFFFFF"  borderColor="#8569CB"  borderWidth=2
独立节点:     fillColor="#FFFFFF"  borderColor="#DEE0E3"  borderWidth=2
```

textColor 规则：
- 正文：`#1F2329`（深色，在白底/浅色底上清晰）
- 辅助：`#646A73`（弱化，不抢注意力）
- 深色底：`#FFFFFF`（反色，清晰可读）

---

## 各元素怎么画

> 以下示例使用默认马卡龙色。如果用户指定了配色，将示例中的颜色替换为用户的颜色，结构保持不变。

### 图表标题

告诉读者"这张图讲什么"。大号深色文字，居中。

```json
{ "type": "text", "fontSize": 24, "textColor": "#1F2329", "textAlign": "center" }
```

### 分区背景

把相关的内容圈在一起，告诉读者"这些属于同一个大类"。浅色做 fillColor，对应深色做 borderColor。内部放白色节点。

```json
{ "fillColor": "#F0F4FC", "borderColor": "#5178C6", "borderWidth": 2, "borderRadius": 8, "padding": 20 }
```

### 分区标签

给分区一个名字。用独立 text 节点，不要用 frame 的 `title` 属性（会被渲染为极小标题栏）。

```json
{ "type": "text", "width": 180, "height": "fit-content", "text": "Access layer", "fontSize": 20, "textColor": "#1F2329", "textAlign": "right" }
```

### 分组标题

告诉读者"这个子分组叫什么"。色板色填充 + 同色系深色边框。

```json
{ "fillColor": "#EAE2FE", "borderColor": "#8569CB", "borderWidth": 2, "borderRadius": 8, "fontSize": 14, "textColor": "#1F2329" }
```

### 内容节点

具体的信息项。白色填充，边框颜色跟随所属分组。

```json
{ "fillColor": "#FFFFFF", "borderColor": "#5178C6", "borderWidth": 2, "borderRadius": 8, "fontSize": 14, "textColor": "#1F2329" }
```

### 表头

告诉读者"这一列/行是什么维度"。深色填充 + 白色文字。

```json
{ "fillColor": "#1F2329", "borderColor": "#1F2329", "borderWidth": 2, "borderRadius": 0, "fontSize": 15, "textColor": "#FFFFFF", "textAlign": "center" }
```

### 辅助说明

补充信息，不抢主角的注意力。灰色小字。

```json
{ "fontSize": 13, "textColor": "#646A73" }
```

### 连线

表达元素之间的关系或流向。灰色或黑色，不用彩色。

```json
{ "lineColor": "#BBBFC4", "lineWidth": 2 }
```

### 布局容器

纯粹用来排版的 frame，读者看不见它。不设 fillColor、borderColor。

```json
{ "type": "frame", "layout": "vertical", "gap": 28, "padding": 32 }
```

### 分组容器

用虚线框圈定一组节点，比分区背景更轻量。

```json
{ "borderColor": "#DEE0E3", "borderWidth": 2, "borderDash": "dashed", "borderRadius": 8 }
```

---

## 常见错误

❌ 每个节点一种颜色 → 读者分不清谁和谁是一组
```json
{ "fillColor": "#8569CB" }, { "fillColor": "#5178C6" }, { "fillColor": "#509863" }
```
✅ 同组节点视觉一致 → 读者一眼看出关系
```json
{ "fillColor": "#FFFFFF", "borderColor": "#8569CB" }, { "fillColor": "#FFFFFF", "borderColor": "#8569CB" }
```

❌ 内外层都用重色 → 读者不知道先看哪里
```json
{ "type": "frame", "fillColor": "#5178C6", "children": [{ "fillColor": "#8569CB" }] }
```
✅ 外层浅色内层白色 → 读者先看结构再看细节
```json
{ "type": "frame", "fillColor": "#F0F4FC", "children": [{ "fillColor": "#FFFFFF", "borderColor": "#5178C6" }] }
```

❌ 连线用彩色 → 和节点颜色抢注意力
```json
{ "connector": { "lineColor": "#5178C6" } }
```
✅ 连线用灰色 → 衬托节点
```json
{ "connector": { "lineColor": "#BBBFC4" } }
```

❌ 节点没边框 → 和背景融为一体，看不清边界
```json
{ "fillColor": "#FFFFFF" }
```
✅ 节点有边框 → 边界清晰
```json
{ "fillColor": "#FFFFFF", "borderColor": "#DEE0E3", "borderWidth": 2 }
```

❌ 全图黑白灰，没有颜色区分 → 读者无法快速识别分组
```json
{ "fillColor": "#FFFFFF", "borderColor": "#DEE0E3" }
```
✅ 不同分组用不同颜色 → 一眼看出结构（蓝色分组 + 紫色分组）
```json
{ "fillColor": "#F0F4FC", "borderColor": "#5178C6" }
{ "fillColor": "#EAE2FE", "borderColor": "#8569CB" }
```

---

## 怎么上色

无论什么类型的图，上色步骤相同：

1. **找出图中有几个分组**（层级、分支、类别、列、阶段……任何可以区分的维度）
2. **为每个分组选一种颜色**（从用户指定的色板或默认马卡龙色板中选；浅色做 fillColor，深色做 borderColor）
3. **分组的容器/区域**用浅色填充 — 告诉读者"这块是一个整体"
4. **分组内的具体节点**用白色/最浅色填充 + 该分组的深色 borderColor — 告诉读者"这些属于这个分组"

适用于所有类型的图：
- 架构图有 3 层 → 每层一种颜色，层背景浅色填充，层内节点白色+深色边框
- 对比表有 3 列 → 每列表头一种颜色，该列数据单元格用同色边框
- 组织架构有 4 个部门 → 每个部门一种颜色，子部门白色+同色边框
- 流程图 → 起止节点一种颜色，判断节点一种颜色，步骤节点白色
