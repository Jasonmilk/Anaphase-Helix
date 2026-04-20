# 改写增强工作流

用户提供已有文档链接或 token，需要改写、润色、补充或重排版时，遵循本工作流。

## 核心方法论 — Code-Act Loop
通过自适应的 **Code-Act Loop** 驱动文档改写，而非固定模板式的工作流。每次任务都循环执行：
1. **Plan（规划）** — 根据用户目标和文档当前状态，评估下一步该做什么
2. **Execute（执行）** — 运行相应的 `lark-cli docs` 命令，或 **spawn** Agent 子任务并行推进
3. **Observe（观察）** — 检查命令输出，验证正确性，核查样式是否达标
4. **Iterate（迭代）** — 如需调整，回到 Plan 继续循环

## 核心原则：精准手术优于全量覆盖

## 工作流程

### 第一波 — 分析 + 画板意图识别（串行）

1. `docs +fetch --api-version v2 --detail with-ids` 获取带 block ID 的文档
2. 系统性评估：结构清晰度、富 block 密度（≥40%）、元素多样性（≥3种）、连续 `<p>` 是否超过 3 段、是否有开头 callout 和章节 `<hr/>`
3. **画板意图识别**：逐章节扫描，按 `lark-doc-style.md`「画板意图识别」表判断哪些段落的信息适合用图表达。记录需要插图的章节（block ID）及推荐的画板类型
4. 向用户简要说明改进计划（包含识别出的画板机会）

### 第二波 — 定向改写（并行 Agent）

5. Spawn Agent 在不重叠的章节上并行改进，各 Agent 收到文档 token 和特定 block ID：（见 `lark-doc-style.md`）
   - 开头适当添加 `<callout>`、重组引言
   - 纯文本转为 `<grid>`/`<table>`/`<whiteboard>`
   - **对第一波识别出的画板候选段落**：简单图直接 `<whiteboard type="mermaid|plantuml">`，复杂图 spawn Agent 使用 **lark-whiteboard** skill
   - 添加流程图、对比分栏等富 block

### 第三波 — 验证（串行）

5. 获取更新后文档，重新检查样式指标
6. 未达标则定向修正，向用户呈现结果

## Agent 子任务要求

Spawn Agent 时必须提供：文档 token、章节范围（标题/block ID）、`lark-doc-xml.md` 和 `lark-doc-style.md` 路径、具体的 `docs +update` command 和 `--block-id`。
