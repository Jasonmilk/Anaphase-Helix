# Anaphase-Helix RNA — 加载协议 v1.0

> **所属方法论**：phyt-DNA v1.0（方法论锚点项目 https://github.com/Jasonmilk/phyt-DNA）
> **对齐知识本体**：v1.0
> **状态**：定稿生效

## 加载协议（三层闭环）

**闭环逻辑**：DNA（宪法）→ 加载 spec（知识本体）→ GROWTH（生长代谢）→ DEPRECATE（死亡退役）→ archive（历史归档）

### 第一层：活跃档案（新会话必读，按顺序加载）

| 顺序 | 文件 | 职责 | 大小 |
|---|---|---|---|
| 1 | `docs/DNA.md` | 宪法：8 条不可变原则 | ~40 行 |
| 2 | `docs/RNA.md` | 导航：本文件 | ~70 行 |
| 3 | `docs/GROWTH.md` | 生长：最近 3 次健康快照 | ~30 行 |
| 4 | `docs/DEPRECATE.md` | 临终：正在退役的功能 | ~30 行 |

**总加载量**：≤180 行，≤3000 tokens。

### 第二层：白皮书分卷（按任务加载）

根据任务关键词匹配对应分卷。只加载与当前任务相关的卷，禁止一次性加载全部。

| 卷名 | 路径 | 锚定原则 |
|---|---|---|
| 生态定位 | `docs/spec/position.md` | 脑手分离 |
| 核心原则 | `docs/spec/principles.md` | 全部 |
| 核心架构 | `docs/spec/architecture.md` | 工作态归 Anaphase |
| 与 Mind 的契约 | `docs/spec/contract.md` | 脑手分离 |
| CI-144 通信协议 | `docs/spec/ci-144.md` | 事件驱动 |
| 生命周期协议 | `docs/spec/lifecycle.md` | 双重熔断 |

### 第三层：历史考古（人类主动请求时加载）

| 档案 | 路径 | 加载触发 |
|---|---|---|
| 生长历史 | `docs/archive/growth/` | 回顾演化时 |
| 死亡历史 | `docs/archive/deprecated/` | 考古旧接口时 |
| 决策历史 | `docs/decisions/` | 追溯"为什么选 A"时 |
| 设计文档 | `docs/design/` | 追溯降级链/安全设计时 |

## AI 协作铁律

1. **注意力锚定**：加载 DNA.md 后，根据当前任务判断最相关的 1-2 条原则，将主要注意力放在它们上。
2. **质疑者**：检查方案是否违背 DNA 原则，违规时告警。
3. **禁止修宪**：不得修改 DNA.md（可提议，人类拥有最终决策权）。
4. **权限检查**：架构/接口变更前，确认当前自治级别是否允许该操作。代理人模式下不得写入 L3。
5. **决策拦截**：架构/接口变更时，提示创建 `docs/decisions/NNNN-xxx.md`。
6. **原子重构**：修改函数签名时，列出所有调用方；≤5 处本轮修复，>5 处生成脚本。
7. **消灭魔法**：禁止硬编码阈值，全部走配置。
8. **认识论螺旋**：理解"只追加不修改"不是简单堆积，而是通过 `CORRECTS` / `REFINES` / `DOUBTS` 辩证边实现时间轴上的螺旋上升。

## 大版本更新 SOP

### 更新前
- [ ] 确认目标：改哪几卷？（见上"白皮书分卷"表）
- [ ] 确认边界：触碰 DNA 哪条原则？
- [ ] 确认遗产：`DEPRECATE.md` 有待安葬项？
- [ ] 切分支：`git checkout -b feature/vX.X-简述`

### 更新中
- [ ] 修改对应 `docs/spec/` 分卷（只改相关卷）
- [ ] 代码实现
- [ ] DNA 原则自检（8 条核心原则是否违背？）

### 更新后
- [ ] 测试通过
- [ ] 写 `GROWTH.md`（记录本次生长健康度）
- [ ] 写 ADR（`docs/decisions/`，如有艰难决策）
- [ ] 写 `DEPRECATE.md`（如有退役功能）
- [ ] 归档：`GROWTH` 超 3 条？移入 `docs/archive/growth/`
- [ ] 归档：`DEPRECATE` 已安葬？移入 `docs/archive/deprecated/`
- [ ] 合并分支 + 删除 feature 分支

## EVOLUTION

- v1.0 (2026-08-28): 从 VISION/SPEC 脱水，三层加载协议定稿，与 Helix-Mind 方法论结构对齐
