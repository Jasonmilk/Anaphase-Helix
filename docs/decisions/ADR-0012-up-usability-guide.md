# ADR-0012: 易用引导 UX（G-5）——`up` 首跑引导

- **状态**: Accepted
- **日期**: 2026-09-06
- **关联**: ADR-0011（bootstrap `up`）、候选 G（驾驶舱）、用户约束（2026-09-06：**"不希望我或者用户需要一大堆 cli 来操作！需要设计引导……易用性真的非常重要，否则看起来就是个人玩具项目"**）
- **仓库**: Anaphase（src/bin/up.rs）

## 1. 背景

G-4 的 `up` 已经一条命令起全栈，但输出是 debug 式（`== Helix backend bootstrap ==` +
裸状态行）。首跑者仍然不知道：缺了什么、为什么空转、下一步做什么。
用户明确"命令记不住"、"不懂用"——引导不是可选项。

## 2. 决策

### D1: 引导 = 信息充分 + 路径清晰，不做交互式 wizard

`up` 保持确定性（非交互），但输出是**引导四段式**：
`欢迎 → 前置检查 → 启动 → 下一步`。首跑者永远知道：什么在跑、什么缺失、
下一步做什么。交互式问答留到 Web UI 时代（G2），TUI/CLI 保持脚本可驱动
（物理事实优先：引导是投影，不是会话）。

### D2: 前置检查 = 纯函数 + 具体构建命令

`check_prereqs()` 返回缺失清单，每项带 `cargo build` 提示（如
`cd helix-tentacle && cargo build`）。可测（3 单测：全就绪/Anaphase 缺失
为致命/缺失项带构建提示）。Anaphase 缺失 = 致命（没有本体无从启动）；
Tentacle/Cellrix 缺失 = fail-open（DNA 铁律 6，不阻塞）。

### D3: 输出用中文（用户母语），代码注释保持英文

运行输出面向 Jasonmilk 本人 + 潜在中文使用者——**使用用户所用语言**；
代码注释/标识符按项目铁律全英文。模式标签中文化（drive 驾驶/partner 伙伴/
survive 生存，ADR-0006）。

### D4: Noop 引导

reasoning 未配置时明确提示"Noop 模式（ledger 为空）" + 配置方式
（`config.toml [anaphase] reasoning_endpoint = "..."`）——消除
"为什么驾驶舱没数据"的疑惑。

## 3. 验证（已通过）

1. `cargo test` 132 passed（129 + 3 up 引导单测，无回归）
2. **正常场景实测**：前置检查全 ok + Tentacle/Anaphase 就绪（中文模式标签）+
   下一步三选项（另起终端开驾驶舱 / 一键重来带驾驶舱 / README）
3. **缺失场景实测**（tentacle 临时改名）：
   `[•] Tentacle 未找到 → cd helix-tentacle && cargo build` +
   `[i] 缺失项可按提示构建；Tentacle/Cellrix 缺失不会阻塞（fail-open）` +
   Anaphase 照常就绪（离线 Noop）——fail-open 物理验证
4. 退出后端口全清（0 残留）

## 4. 后果

**正面**：首跑者零困惑——缺什么、怎么补、下一步做什么全部在屏；
`up` 成为真正的"唯一入口"（banner 即帮助，无需背命令）。

**代价/缺口**：
- 引导是文本投影，不含交互式首次配置向导（wizard）——已记录到
  G2（Web 面板）候选；TUI 引导当前满足"不困惑"目标。
- 下一步建议里的驾驶舱命令较长（含 mock-agent 路径）——未来
  `up` 加 `--cockpit` 的等价子命令（如 `up cockpit`）可收敛，属增强项。
