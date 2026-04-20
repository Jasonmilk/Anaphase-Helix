# 流程图

> 本文件的 DSL 示例仅展示结构和布局，使用中性占位色。实际颜色、边框、字号等视觉属性请按当前加载的 style 文件的视觉角色定义应用。

适用于：业务流程、审批流、订单处理流程等**有明确顺序和分支判断**的场景。

## 结构特征

遵循标准流程图符号规范（ISO 5807）：
- **起止节点**：圆角矩形（rect + borderRadius）— 对应 Terminator
- **处理步骤**：矩形（rect）— 对应 Process
- **判断节点**：菱形（diamond）— 对应 Decision

节点之间用 connector 连接，箭头表示流程走向。分支处的连线通常需要标注条件（如"是/否"）。

## 布局策略

**线性流程**：使用 Flex 纵向布局（`layout: 'vertical'`），节点从上到下依次排列。

**带分支的流程**：使用混合布局。主干用纵向 Flex，分支处用横向展开：
- 主干保持垂直方向，自上而下
- 判断节点（diamond）后，"是"继续主干向下，"否"分支向右/左侧展开
- 分支路径最终汇合回主干

**关键参数**：
- 节点间 gap: >= 40px，为连线留出空间
- 判断节点（diamond）需要设置足够的 width/height（如 120x80），确保文字不截断
- 连线使用 `endArrow: 'arrow'` 标明方向

## DSL 结构示例

线性审批流程：

```json
{
  "version": 2,
  "nodes": [
    {
      "type": "frame",
      "width": 800,
      "height": "fit-content",
      "layout": "vertical",
      "gap": 40,
      "padding": 40,
      "alignItems": "center",
      "children": [
        {
          "type": "rect",
          "id": "start",
          "width": 200,
          "height": "fit-content",
          "fillColor": "#FFFFFF",
          "borderColor": "#DEE0E3",
          "borderWidth": 2,
          "borderRadius": 8,
          "text": "开始",
          "fontSize": 16,
          "textColor": "#1F2329",
          "textAlign": "center",
          "verticalAlign": "middle"
        },
        {
          "type": "rect",
          "id": "submit",
          "width": 200,
          "height": "fit-content",
          "fillColor": "#FFFFFF",
          "borderColor": "#DEE0E3",
          "borderWidth": 2,
          "borderRadius": 8,
          "text": "提交申请",
          "fontSize": 14,
          "textColor": "#1F2329",
          "textAlign": "center",
          "verticalAlign": "middle"
        },
        {
          "type": "diamond",
          "id": "check",
          "width": 160,
          "height": 80,
          "fillColor": "#FFFFFF",
          "borderColor": "#DEE0E3",
          "borderWidth": 2,
          "text": "审批通过？",
          "fontSize": 14,
          "textColor": "#1F2329",
          "textAlign": "center",
          "verticalAlign": "middle"
        },
        {
          "type": "rect",
          "id": "approve",
          "width": 200,
          "height": "fit-content",
          "fillColor": "#FFFFFF",
          "borderColor": "#DEE0E3",
          "borderWidth": 2,
          "borderRadius": 8,
          "text": "审批通过\n发送通知",
          "fontSize": 14,
          "textColor": "#1F2329",
          "textAlign": "center",
          "verticalAlign": "middle"
        },
        {
          "type": "rect",
          "id": "end",
          "width": 200,
          "height": "fit-content",
          "fillColor": "#FFFFFF",
          "borderColor": "#DEE0E3",
          "borderWidth": 2,
          "borderRadius": 8,
          "text": "结束",
          "fontSize": 16,
          "textColor": "#1F2329",
          "textAlign": "center",
          "verticalAlign": "middle"
        }
      ]
    },
    {
      "type": "rect",
      "id": "reject",
      "x": 600,
      "y": 260,
      "width": 160,
      "height": "fit-content",
      "fillColor": "#FFFFFF",
      "borderColor": "#DEE0E3",
      "borderWidth": 2,
      "borderRadius": 8,
      "text": "驳回\n退回修改",
      "fontSize": 14,
      "textColor": "#1F2329",
      "textAlign": "center",
      "verticalAlign": "middle"
    },
    {
      "type": "connector",
      "connector": {
        "from": "start",
        "to": "submit",
        "fromAnchor": "bottom",
        "toAnchor": "top",
        "lineShape": "straight",
        "lineColor": "#BBBFC4",
        "lineWidth": 2,
        "endArrow": "arrow"
      }
    },
    {
      "type": "connector",
      "connector": {
        "from": "submit",
        "to": "check",
        "fromAnchor": "bottom",
        "toAnchor": "top",
        "lineShape": "straight",
        "lineColor": "#BBBFC4",
        "lineWidth": 2,
        "endArrow": "arrow"
      }
    },
    {
      "type": "connector",
      "connector": {
        "from": "check",
        "to": "approve",
        "fromAnchor": "bottom",
        "toAnchor": "top",
        "lineShape": "straight",
        "lineColor": "#BBBFC4",
        "lineWidth": 2,
        "endArrow": "arrow"
      }
    },
    {
      "type": "connector",
      "connector": {
        "from": "check",
        "to": "reject",
        "fromAnchor": "right",
        "toAnchor": "left",
        "lineShape": "straight",
        "lineColor": "#BBBFC4",
        "lineWidth": 2,
        "endArrow": "arrow"
      }
    },
    {
      "type": "connector",
      "connector": {
        "from": "reject",
        "to": "submit",
        "fromAnchor": "top",
        "toAnchor": "right",
        "lineShape": "rightAngle",
        "lineColor": "#BBBFC4",
        "lineWidth": 2,
        "lineStyle": "dashed",
        "endArrow": "arrow"
      }
    },
    {
      "type": "connector",
      "connector": {
        "from": "approve",
        "to": "end",
        "fromAnchor": "bottom",
        "toAnchor": "top",
        "lineShape": "straight",
        "lineColor": "#BBBFC4",
        "lineWidth": 2,
        "endArrow": "arrow"
      }
    }
  ]
}
```

## 陷阱与检查项

- **连线锚点方向必须匹配流向**：主干自上而下用 `bottom→top`；分支向右侧展开用 `right→left`；回退连线（驳回后退回上级步骤）用 `top→right` 或 `top→bottom` 配合 `lineStyle: 'dashed'` 以示区分。
- **diamond 节点不支持 fit-content 高度**：菱形的内接矩形面积有限，文字容易超出可视区域。建议设置足够的固定宽高（如 160x80），文字控制在 4-6 个字以内。
- **分支节点脱离 Flex**：如果主干用 Flex 纵向布局，分支节点（如"驳回"）需要放在 Flex 容器外部，使用绝对定位（x/y），否则会被插入主干队列中。
- **回退连线交叉**：驳回回退的连线容易与主干连线交叉。使用 `rightAngle` 或 `polyline` 线型绕行，避免视觉混乱。
- **缺少箭头**：流程图的每条连线都应该有 `endArrow: 'arrow'`，否则无法看出方向。
