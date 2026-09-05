# ADR-0011: 一条命令起全栈（bootstrap）——`up` 与 env 覆盖

- **状态**: Accepted
- **日期**: 2026-09-06
- **关联**: 候选 G（驾驶舱）、ADR-0002（零硬编码）、ADR-0004（Tentacle 默认端口）、G-3（transport 契约修复）
- **仓库**: Anaphase（src/bin/up.rs + src/config.rs env 覆盖）

## 1. 背景

驾驶舱 G-3 修复后，跑通一次真实演示需要 3-4 条 CLI：起 tentacle（带
`--plugins-dir`）、配 anaphase config、起 cellrix-cli（带 --mode/--exec/
--anaphase-endpoint）。用户明确：**"我不希望我或者用户需要一大堆 cli 来操作！
需要设计引导……易用性真的非常重要，否则看起来就是个人玩具项目。"**
（2026-09-06）

## 2. 决策

### D1: 引导 = Anaphase 的一个 bin（`up`），不新增 crate

Anaphase 是意识层/编排方，拉起生态是它的职责；`src/bin/up.rs` 零新实体、
零新依赖（仅 std + 复用 crate 内 config）。

### D2: 注入配置用 env 覆盖，不写 config.toml

`up` 通过 `ANAPHASE_TENTACLE_ENDPOINT`/`ANAPHASE_REASONING_ENDPOINT` 注入
Anaphase（12-factor：env 优先于文件）。理由：
- **无并发写文件风险**（config.toml 是用户基态，写它有崩溃/竞争风险）
- **可逆**（env 是调用方视图，进程结束即消失）
- **显式**（进程级可见，可审计）
config.rs 新增 `apply_env_overrides`（env 非空才覆盖，空值忽略 = fail-open），
3 个单测（串行锁防 env 竞争）。

### D3: 一切端口/路径从配置或协议默认派生（零硬编码）

| 项 | 来源 |
|---|---|
| tentacle 二进制 | `HELIX_TENTACLE`（显式）→ 工作区布局 `../helix-tentacle/target/debug/tentacle` |
| fixtures 目录 | `HELIX_FIXTURES_DIR` → 工作区布局 `../helix-tentacle/fixtures` |
| grpc 端口 | config.toml `tentacle_endpoint` URL 解析 → `HELIX_TENTACLE_PORT` → **协议默认 50051**（ADR-0004 文档化） |
| snapshot 端口 | config.toml `cap_http_port` |
| 就绪超时 | 协议默认 const（注释来源） |

### D4: fail-open（DNA 铁律 6）

tentacle 二进制缺失 → warn + 降级 Noop（离线模式），不阻塞 anaphase 启动。
探测 = 物理事实（TCP 端口可达），不猜日志。

### D5: 退出 = 前台进程组共同收 SIGINT

`up` 用 `Command::spawn` 起子进程（同一前台进程组），终端 Ctrl+C 同时送达
全栈；`wait_for_signal` 仅保持父进程存活。无守护进程、无 PID 文件（边界：
M3 禁止清单遵守）。

## 3. 验证（已通过）

1. `cargo test` 129 passed（126 + 3 env 覆盖单测，全绿无回归）
2. 实测 `./target/debug/up`：
   ```
   == Helix backend bootstrap ==
   tentacle:  .../helix-tentacle/target/debug/tentacle (grpc :50051, fixtures: .../fixtures)
   anaphase:  config.toml + ANAPHASE_TENTACLE_ENDPOINT=grpc://127.0.0.1:50051
     [ok] tentacle grpc ready on :50051
     [ok] anaphase snapshot ready on :50061
   == stack up ==
   ```
3. 退出后端口全清（50051/50061 无残留）
4. `--cockpit` 模式：拉起驾驶舱 TUI（stdio transport + mock-agent + snapshot 端点）

## 4. 后果

**正面**：演示/自用从 4 条 CLI 收敛为 1 条；env 覆盖成为 Anaphase 的标准
注入通道（后续 reasoning/mind 端点同模式扩展）；fail-open 保证无二进制
也永不阻塞。

**代价/缺口**：
- 引导尚未做**引导式 UX**（用户提示/欢迎语/首跑向导）——下阶段
  （易用引导候选 G-5）补；本轮只保证"一条命令跑通"。
- cockpit 依赖 Cellrix 已构建的二进制；未构建时 warn 提示（不自动构建，
  构建是开发者动作，非运行动作）。
- 进程组退出依赖真实终端 Ctrl+C；脚本场景（python send_signal 单进程）
  需手动清理子进程——记录为已知行为，非缺陷。
