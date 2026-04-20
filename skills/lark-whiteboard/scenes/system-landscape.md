# 混合布局（岛屿式）指南

> 本文件的 DSL 示例仅展示结构和布局，使用中性占位色。实际颜色、边框、字号等视觉属性请按当前加载的 style 文件的视觉角色定义应用。

适用于：系统集成图、飞轮图、模块关系图等**没有明确层级、多模块平级互联**的场景。

## 核心思路

```
宏观（Macro）：绝对定位（x/y）自由摆放各模块（岛屿）
微观（Micro）：每个模块内部用 layout: 'vertical'/'horizontal' 自动对齐
背景域（可选）：大虚线框划分逻辑分区
连线：connector 表达模块间关系
```

## 拓扑探测与排版策略 (Topological Strategy)

在分配 `x/y` 坐标前，**必须**先根据业务语义决定拓扑结构，千万避免"所有图都画成死板的网格阵列"或"强行绕圈圈"：

1. **判断图表类型**：
   - **分层架构图**（如经典前端->网关->微服务）：必须使用**严格的网格对齐**（Grid），坐标横平竖直。
   - **业务流程图/逻辑流转**（如流水线、重试链路）：使用**流向驱动的自由排版（Flow-driven）**。顺着人类阅读动线（Z型、S型、由左及右）铺开，不强求死板对齐。

2. **确立核心节点**：找出核心中枢节点或图表标题，将其放在画布中心或轴线上，其他节点围绕主干线展开。

3. **空间位置的语义约束**：
   - 基本流源头在左/上，终点在右/下。
   - **保主干清晰，旁支让路**：旁路节点（如异常重试）、边缘操作（如归档、丢弃、补漏）**绝不要阻断或横跨**主干链路。将它们依附于触发节点就近摆放，向无人的边缘（如右侧翼、外部白空间）发散，给原本顺畅的主链路留出足够的空间。

## 尺寸预估定稿

1. **先计算内容**：岛屿内部用 `fit-content` 让内容撑开。
2. **留白预留**：如果要画大 S 曲线或复杂连线，两节点间的预估间距要放到 **150px-250px**。不要一味缩减到 60px。
3. **摆放坐标**：如果是流向驱动，按顺流坐标逐步确定 x 和 y，不要有"为了左右逢源而强行对齐列宽"的强迫症。
4. **如果需要背景域**：必须声明在其他图元之前（处理 z-index），`layout: 'none'` 下必须写死绝对像素长宽。

## 模板结构

模板结构分为四个区块：标题（顶部 text 节点）→ 背景域（可选，虚线 frame，先声明以处于底层 z-index，标签用独立 text 节点）→ 岛屿模块（绝对定位 + 内部 flex，可有多个）→ 连线（必须放顶层 nodes）。

```json
{
  "version": 2,
  "nodes": [
    { "type": "text", "x": 350, "y": 25, "width": 800, "height": "fit-content",
      "text": "图表标题", "fontSize": 24, "textColor": "#1F2329", "textAlign": "center" },

    { "type": "frame", "x": 20, "y": 110, "width": 1460, "height": 490,
      "layout": "none", "gap": 0, "padding": 0,
      "fillColor": "#FFFFFF", "borderColor": "#DEE0E3",
      "borderDash": "dashed", "borderWidth": 2, "borderRadius": 8 },
    { "type": "text", "x": 30, "y": 115, "width": 200, "height": "fit-content",
      "text": "逻辑分区名", "fontSize": 16, "textColor": "#1F2329" },

    { "type": "frame", "id": "module-a", "x": 50, "y": 170,
      "width": 390, "height": "fit-content",
      "layout": "vertical", "gap": 12, "padding": 20,
      "fillColor": "#FFFFFF", "borderColor": "#DEE0E3",
      "borderWidth": 2, "borderRadius": 8,
      "children": [
        { "type": "text", "width": "fill-container", "height": "fit-content",
          "text": "模块标题", "fontSize": 20, "textColor": "#1F2329", "textAlign": "center" },
        { "type": "rect", "width": "fill-container", "height": "fit-content",
          "text": "子组件\n描述信息", "fontSize": 13, "textColor": "#1F2329",
          "fillColor": "#FFFFFF", "borderColor": "#DEE0E3", "borderWidth": 2, "borderRadius": 8,
          "textAlign": "center", "verticalAlign": "middle" }
      ]
    },

    { "type": "connector", "connector": {
      "from": "module-a", "to": "module-b",
      "fromAnchor": "right", "toAnchor": "left",
      "lineShape": "straight", "lineColor": "#BBBFC4", "lineWidth": 2,
      "endArrow": "arrow", "startArrow": "arrow"
    }}
  ]
}
```
