# ADR-0025: L0-L3 全层打通 + 工具链确定性执行（认知层实弹）

- **状态**: Accepted
- **日期**: 2026-09-07
- **决策范围**: Anaphase（identity 注入 / 调用解析 / 回复回填）、Helix-Mind（remember 层路由）、Helix-Tentacle（calc / web_search 工具）
- **关联**: ADR-0001（Mind 触发链）、ADR-0002（DNA 原则 11）、ADR-0021（认知工艺）、ADR-0024（判断点后端）

## 1. 背景与问题

用户逐层核查 L0-L3 是否真连：
1. **L0 基因锁未生效**——问"你姓什么"答"没有姓"：identity 从未进 prompt；
2. **L1 自画像缺失**——Helix 不知道自己能用什么工具、按什么哲学工作；
3. **工具链未真执行**——大数次方必须用工具算、不许口算；LLM 输出 Markdown ```tool 围栏（非 JSON）导致解析失败；
4. **L2 知识层空**——nodes: L1|19、L3|52、L2|0、edges 0——没有写入路径；
5. **搜索能力缺失**——"有疑问可以搜索，用 tentacle 搜索也行"。

## 2. 决策

### D1: identity_block——L0+L1 合成注入（极致复用）
`build_identity_block()`（main.rs）读 `gene_lock.md`（Lineage=Dash）+ 拉取 Tentacle `ListManifests`，合成：
```
[identity ...]        ← L0 基因锁（姓、血统）
[tools available ...] ← L1 工具清单（calc/web_search/rate/numbers…）
[philosophy]          ← 按需获取/按需加载/工具优先/不许口算
[protocol]            ← "用工具时以 ONLY JSON {"calls":[...]} 结尾"
```
单一块、单次注入、单一来源（gene_lock 文件 + Tentacle manifest），零硬编码。

### D2: 调用解析三层兜底（确定性优先）
1. 严格 JSON（`{"calls":[...]}`）→ 首选；
2. ```tool 围栏（LLM 因 Markdown 习惯输出）→ `parse_tool_fence()`：
   `name(key="value")` / `name({json})` / `name("value")` 三种形态；
3. 无调用 → 普通回合（echo 兼容保留，向后兼容 228 测试）。
**效果**：LLM 输出形态不再决定成败，解析器兜住形态差。

### D3: 回复回填——用户看到结果，不是计划（物理事实优先）
Reflection 分支把 evidence 拼成 `"{tool}: {data}"` 覆盖 reasoning_output。
实测：`7**9` → `calc: {"ok":true,"result":"40353607"}`。
**哲学**：交作业给结果，不给"我打算做什么"。

### D4: L2 知识层写入——remember_node（按需驱动）
- Mind proto：`RememberRequest.node_type`（-1/缺省=协议默认 L3；0..=3=L0..L3）；
- Anaphase adapter 新增 `remember_node(content, layer)`（trait 默认实现回退 remember，Noop 忽略非 L3）；
- **层语义由 Mind 执行，命名由编排方发出**（极致解耦：Anaphase 只写意图，Mind 只认层）。
- 种子：熵增定律 / 质量守恒 / 万有引力（每条 = 主张 + 适用边界，Helix 引用边不发明边）。

### D5: web_search——确定性只读搜索
Tentacle 插件：固定 Bing HTML 端点 + 白名单正则提取 `b_algo` 标题/链接，
参数 `q`（≤200 字符）+ `max`（默认 5，封顶 8），SHA-256 校验。
**边界**：工具只取数不判义——相关性判断留给认知工艺（批判性思维是 Helix 的工序）。

### D6: 回复协议强制（后续观察项）
曾出现 LLM 用 ```tool 而非 JSON——协议行已进 identity_block。
若再复发：协议提示升级为 prompt 断言（第二行重复 ONLY JSON）。

## 3. 备选与拒绝

| 备选 | 拒绝理由 |
|---|---|
| 直接改 LLM system prompt 强制 JSON | identity_block 已含协议行；解析兜底更稳（不赌 LLM 听话） |
| L2 直写 SQLite | 绕过协议层=破坏分层；必须经 Mind gRPC 写入 |
| 搜索用完整浏览器抓取 | 重、慢、易碎；固定端点+白名单解析足够（极致节能） |

## 4. 后果

**正面**：
- L0-L3 全层实弹（问"你姓什么"→Dash；L2 三条定律入库可检索）；
- 工具链确定性闭环：LLM JSON → pipeline → Tentacle gRPC → node 执行 → evidence 回填；
- 231 tests 全绿（+3 围栏单测），双仓（anaphase 231 / tentacle 153+）零失败。

**代价/待办**：
- web_search 结果质量（Bing HTML 首条偶现泛化结果）→ 解析规则微调挂 GROWTH；
- 搜索/工具协议行依赖 LLM 遵循，若再违约升级 prompt 断言。

## 5. 一句话总结

> L0 定姓、L1 亮剑、L2 立知、L3 记事；
> 工具是手，搜索是眼，边界是德——Helix 只走确定性的路。
