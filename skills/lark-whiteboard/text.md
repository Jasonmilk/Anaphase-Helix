# 文字排版与截断防范

## 容器高度用 fit-content，不要写死

```typescript
// ❌ 错误：容器写死高度 → 文字截断
{ type: 'frame', height: 60, children: [
  { type: 'text', text: '很长的文字\n会换行' }  // 超出 60px 的部分会被无情截断！
]}

// ✅ 正确：容器高度 fit-content
{ type: 'frame', height: 'fit-content', children: [
  { type: 'text', text: '很长的文字\n会换行' }  // 自动根据文字撑开
]}
```

---

## 🚨 尺寸与文字截断反模式（极易犯错！） 🚨

> [!CAUTION]
> **禁止主观猜测固定宽高！** 模型最容易犯的错误就是瞎编固定尺寸（例如 `width: 60, height: 40`），导致文字换行后直接被切掉一半，例如 `A\n(已售)`。
> 只要内部有文字（尤其是含有 `\n` 或多行时），**绝对不要写死高度**，请全部交给引擎自适应！

> [!IMPORTANT]
> **规则 0：所有节点（尤其是 Frame 容器），必须显式声明 `width` 和 `height`！**
> 如果任何包裹元素的容器不填宽高，引擎将**无法正确计算该节点的边界**，导致渲染越界、排版重叠。
> **强制兜底写法：只要你声明了一个节点，没有明确宽高时无脑加上 `width: 'fit-content', height: 'fit-content'`。**

> [!IMPORTANT]
> **规则 1：绝对禁止写死含文字节点的高度（不要猜高度）。**
> 含文字的 shape 或 frame，`height` **必须始终使用** `'fit-content'`。fit-content 已内含 12px 边距，绝对不会截断。
> ❌ 反模式：`{ type: 'rect', width: 65, height: 45, text: 'A\n(已售)' }`（绝对会截断！）
> ✅ 正确：`{ type: 'rect', width: 'fit-content', height: 'fit-content', text: 'A\n(已售)' }`（引擎会完美包住文字）

> [!IMPORTANT]
> **规则 2：如何让一排卡片"等大"？（禁止写死同样的固定宽/高）**
> 当你想实现一排座位、一排卡片"等宽等高"时，**不要试图给每个子节点写同样的绝对像素值去对齐**。
> ✅ **终极解法：** 父节点 `layout: 'horizontal', alignItems: 'stretch'` + 所有子节点 `width: 'fill-container', height: 'fit-content'`。
> 靠 Flex 的威力，最高的子元素撑开整排，其余矮的自动拉高，宽的自动平分！
>
> **注意：** Shape 节点（rect、ellipse、diamond 等）的内部文字**默认水平居中 + 垂直居中**（即默认 `textAlign: 'center'`, `verticalAlign: 'middle'`），与 CSS 默认行为相反。如需左对齐或顶部对齐，须显式声明。

> [!IMPORTANT]
> **规则 3：一定要猜固定尺寸时，请做加法计算。**
> Shape 节点内部有强制内边距，文字不能画到边缘。fit-content 会自动处理各种形状（含椭圆、菱形、三角形）的几何补偿，无需手动计算。
> - **rect / ellipse / diamond / triangle**：上下左右各 12px（TEXT_INSET），即垂直 +24px、水平 +24px
> - **cylinder**：桶盖弧形占顶部 32px、底部弧形占 10px，即垂直 **+42px**；水平各 7px 即 +14px
>
> 最小可用物理尺寸公式（仅在必须用固定高度时参考，优先用 `fit-content` 自动计算）：
> `实际文字宽/高 + 对应 inset`。
> 例如 rect 里 12px 字号两行文字高 28px → `height ≥ 28 + 24 = 52px`；
> 同样内容放 cylinder → `height ≥ 28 + 42 = 70px`（桶盖占了更多空间）。
> **cylinder 必须用固定宽度（不能 fill-container）+ `height: "fit-content"`**，详见 `dsl.md`。

```typescript
// ❌ 固定高度 → 文字折行就截断（反模式）
{ type: 'rect', width: 150, height: 60, text: '新都桥\n摄影家天堂 · 海拔 3460m' }

// ✅ fit-content → 高度自适应（正确做法）
{ type: 'rect', width: 150, height: 'fit-content', text: '新都桥\n摄影家天堂 · 海拔 3460m' }

// ❌ 瞎猜固定尺寸太小 → 文字被裁或折行（40 − 24 = 只剩 16px 放文字）
{ type: 'rect', width: 60, height: 40, text: 'A\n(已售)', fontSize: 13 }

// ✅ 完美的等大魔法（靠父级 flex 撑起）
{ type: 'frame', layout: 'horizontal', alignItems: 'stretch', children: [
  { type: 'rect', width: 'fill-container', height: 'fit-content', text: 'A\n(已售)', ... }
]}
```

---

## 文字层级与排版

> [!IMPORTANT]
> **规则 4：字号必须有层级区分。**
> 同一张图中，读者需要快速分辨标题、内容、注释。至少用 2 个不同字号，相邻层级字号差 >= 6px。同级节点的 fontSize 必须完全相同。

```
❌ 所有节点 fontSize: 14 → 读者分不清主次

✅ 标题 24 + 内容 14（差 10px，两级清晰）
✅ 标题 24 + 内容 14 + 注释 13（三级）
```

> [!IMPORTANT]
> **规则 5：对齐方式跟随内容类型。**
> Shape 内短文本默认 `center`（shape 的默认值），不需要显式写。以下场景必须显式指定：
> - 侧标签（架构层名、分区名）：`textAlign: "right"`——右对齐贴近内容区，视觉关联更强
> - 多行描述/段落文字：`textAlign: "left"`——左对齐符合阅读习惯
> - 图表标题：`textAlign: "center"`——居中统领全图

> [!IMPORTANT]
> **规则 6：图表标题用独立 text 节点。**
> 不要用 frame 的 `title` 属性（渲染为极小标题栏）。
> - Flex 布局：放在最外层 frame 的第一个 child，`width: "fill-container"`
> - 绝对定位：width 设为图表整体宽度（如图表区域 1000px 宽则标题也 1000px），用 `textAlign: "center"` 居中。**禁止手算窄宽度 + x 偏移来居中**——标题文字长度不确定，窄宽度会导致换行。

```json
{ "type": "text", "width": "fill-container", "height": "fit-content",
  "text": "图表标题", "fontSize": 24, "textColor": "#1F2329", "textAlign": "center" }
```

> [!IMPORTANT]
> **规则 7：标题和描述拆成两个节点。**
> 一个卡片/模块内需要同时展示名称和描述时，用两个节点分别承载。

❌ 一个 rect 塞所有文字，标题和描述无法区分：

```json
{ "type": "rect", "text": "用户服务\n处理注册登录和个人信息管理", "fontSize": 14 }
```

✅ frame 包两个节点，名称和描述有字号差：

```json
{ "type": "frame", "layout": "vertical", "gap": 4, "padding": 12,
  "width": "fill-container", "height": "fit-content",
  "fillColor": "#FFFFFF", "borderColor": "#DEE0E3", "borderWidth": 2, "borderRadius": 8,
  "children": [
    { "type": "text", "width": "fill-container", "height": "fit-content",
      "text": "用户服务", "fontSize": 16, "textColor": "#1F2329" },
    { "type": "text", "width": "fill-container", "height": "fit-content",
      "text": "处理注册登录和个人信息管理", "fontSize": 13, "textColor": "#646A73" }
  ]
}
```
