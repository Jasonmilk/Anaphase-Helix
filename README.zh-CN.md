# Anaphase-Helix v0.3.2

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.11+](https://img.shields.io/badge/python-3.11+-blue.svg)](https://www.python.org/downloads/)
[![EN](https://img.shields.io/badge/English-README-blue)](./README.md)

[Helix Ecosystem](https://github.com/Jasonmilk) ·
[CIS](https://github.com/CommonIntents/CIS) ·
[CAP](https://github.com/CommonIntents/CAP) ·
[CISS](https://github.com/CommonIntents/CISS) ·
[CIB](https://github.com/CommonIntents/CIB)

**Anaphase-Helix** 是 Helix 生态的执行编排中枢——一个自进化的数字生命体。它协调感知（Tentacle）、记忆（Mind）与推理，通过状态图驱动的 Agent Loop 完成复杂任务。

> **当前状态**：v0.3.2 – Helix-Callosum 桥接已集成。Cellrix CIS 集成已完成。Anaphase 现在可以将认知模式映射到 Callosum 原子参数，通过 HTTP 头部实现确定性 KV Cache 优化。全模块 Mock 模式默认开启，生产模式对缺失依赖进行快速失败。所有脑区已支持通过 Tuck 进行真实 LLM 调用。

## 核心哲学

- **编排优先，拒造实体** – 核心只做调度；实质性工作委托给外部 CLI 工具或微服务。
- **契约至上** – 所有跨模块通信使用严格的 Pydantic DTO。
- **DAG 化一切** – 知识、任务、工具、记忆均建模为有向无环图。
- **引导而非阻断** – Anaphase 以劝导为主；Tuck 作为最后防线执行遏制。
- **硅基代谢** – 主动管理 Token 预算与认知负荷；Agent 在疲劳时进入"睡眠"。
- **纯净 I/O** – `stdout` 专用于数据契约（Manifest JSON、LLM 回复）；`stderr` 仅承载诊断信息。

## 快速开始

### 环境要求

- Python 3.11+
- [Tuck 网关](https://github.com/Jasonmilk/Tuck)（可选，用于真实 LLM 调用）
- [Cellrix](https://github.com/Jasonmilk/Cellrix)（可选，用于交互式认知仪表盘）
- [Helix-Callosum](https://github.com/Jasonmilk/Helix-Callosum)（可选，用于确定性 KV Cache 优化）

### 安装

```bash
git clone https://github.com/Jasonmilk/Anaphase-Helix.git
cd Anaphase-Helix
git checkout V5
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -e ".[dev]"
```

### 零配置启动

Anaphase 默认以 Mock 模式运行，无需 `.env` 文件，也无需任何外部服务：

```bash
ana run "人生的意义是什么？"
```

Agent 将使用 Mock 响应执行完整的七状态认知循环。所有认知日志输出到 `stderr`——终端保持干净。

### 生产模式

当你准备好接入真实 LLM 和记忆节点时，创建 `.env` 文件：

```ini
ANA_MOCK_MODE=false
TUCK_ENDPOINT=http://your-tuck-host:8686
TUCK_API_KEY=你的_API_密钥
TUCK_CHAT_PATH=/v1/chat/completions
HELIX_MIND_ENDPOINT=http://your-mind-host:8020
```

生产模式缺少任何必需变量都会立即触发明确的错误提示——绝不静默失败。

### Helix-Callosum 集成

启用 Callosum 以自动向 LLM 请求注入 KV Cache 优化参数：

```ini
ANA_CALLOSUM_ENABLED=true
```

启用后，Anaphase 将其认知模式映射到 Callosum 原子参数：
- 左脑模型 → `cache_strategy: aggressive`, `temperature: 0.0`
- 右脑模型 → `cache_strategy: isolated`, `temperature: 0.9`
- 杏仁核 → `cache_strategy: balanced`, `temperature: 0.0`

无需额外依赖——桥接对 Anaphase 是原生的。

### 模型配置

确保模型名称与你的 Tuck 实例中可用的一致：

```ini
ANA_AMYGDALA_MODEL=Qwen3.5-2B-IQ4_NL.gguf
ANA_LEFT_BRAIN_MODEL=Qwen2.5.1-Coder-7B-Instruct-Q4_K_M.gguf
ANA_RIGHT_BRAIN_MODEL=DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf
```

## 可视化认知过程

Anaphase 遵循 [Cellrix Intents Specification (CIS)](https://github.com/Jasonmilk/Cellrix/blob/main/CIS.md)。项目根目录中的 `cellrix_manifest.json` 将 Anaphase 声明为意图生产者——无需在 Anaphase 中安装 Cellrix 依赖。通信通过 `stdout` 上的纯 JSON 进行。

### 第一层：验证桥接

```bash
cellrix check
```

预期输出：

```
🔧 Executing bridge command: ana loom --last --cellrix
✅ Bridge executed successfully. Manifest is valid.
```

### 第二层：终端仪表盘

```bash
ana loom --last --cellrix > session.json
cellrix preview session.json
```

你将看到一个包含状态流转图、关键指标和事件时间线的三面板交互式仪表盘。

### 第三层：全屏交互式工作台

```bash
cellrix run -- ana loom --last --cellrix
```

启动基于 Textual 的全屏工作台，使用原生控件（进度条、数据表格）。

### 交互控制

| 快捷键 | 操作 |
|:---|:---|
| `Tab` / `Shift+Tab` | 在面板间循环焦点（聚焦面板高亮为绿色） |
| `F1` | 全屏帮助，显示所有快捷键 |
| `?` | 上下文感知的快捷键参考 |
| `g` | Leader 键——然后按 `a`‑`z` 跳转到指定面板 |
| `↑↓ PgUp PgDn` | 滚动聚焦的面板 |
| `q` | 退出预览（终端完全恢复） |

无需任何配置——这些功能开箱即用。

### 第四层：原有的 Rich 渲染

如果你更喜欢基于 Rich 的原有渲染？它仍然可用：

```bash
ana loom --last
```

## 项目结构

```
Anaphase-Helix/
├── ana/
│   ├── cli/                  # CLI 入口（ana run/trace/stats/loom）
│   ├── core/                 # 脑区模块
│   │   ├── agent_loop.py     # 状态图驱动主循环
│   │   ├── amygdala.py       # 优先级与情感评估
│   │   ├── prefrontal.py     # 推理与规划
│   │   ├── synapse.py        # 工具执行（CLI 安全沙箱）
│   │   ├── commissure.py     # 意图-执行对齐校验器
│   │   ├── callosum_adapter.py # Callosum 桥接
│   │   └── model_router.py   # 基于优先级选择模型
│   ├── loom/                 # Ana Loom 认知可视化
│   │   ├── cellrix_bridge.py # HXR → Cellrix Manifest 编译器
│   │   ├── visualizer.py     # 原有的 Rich 渲染引擎
│   │   └── themes.py         # Ana 主题色彩系统
│   ├── schemas/              # Pydantic DTO 契约
│   ├── common/               # 配置、日志、追踪、重试
│   └── registry/             # 工具注册表
├── config/                   # 基因锁、工具清单、生境模板
├── knowledge_base/           # L1 自画像（self.md）
├── tests/                    # 单元测试（13 个已通过）
├── docs/                     # 白皮书与工程手册
├── cellrix_manifest.json     # CIS 意图生产者声明
├── .env.example
├── pyproject.toml
└── README.md
```

## 测试

```bash
pytest -v
```

全部 13 个测试通过，覆盖所有脑区模块及 Agent Loop 集成。Mock 模式测试验证 DTO 契约与状态转换；真实模式测试（使用 Mock 的 Tuck）验证 HTTP 调用层。

## 文档

- [Helix 生态统一白皮书](docs/WHITEPAPER.md) – "是什么"与"为什么"。
- [Anaphase-Helix 工程手册](docs/ENGINEERING.md) – "怎么做"：项目结构、DTO、AI Coder 铁律及工作流。
- [Helix-Callosum](https://github.com/Jasonmilk/Helix-Callosum) – 上下文内存分配器：确定性 KV Cache 复用与认知桥接。
- [Cellrix](https://github.com/Jasonmilk/Cellrix) – 意图驱动的终端 UI 协议，用于渲染 Anaphase 的认知仪表盘。

## AI Coder 协作

本项目遵循严格的 **AI Coder 铁律清单**（参见工程手册 §7.1），确保 LLM 生成的代码保持一致性、可测试性，并与 Helix 哲学对齐。每个模块均采用测试先行开发，Mock 实现验证通过后再接入真实后端。

## 路线图

| 里程碑 | 状态 |
|:---|:---|
| **v0.1.0** – 物理骨架（目录、DTO、CLI） | ✅ 已完成 |
| **v0.2.0** – 全模块 Mock 集成与端到端验证 | ✅ 已完成 |
| **v0.2.1** – Ana Loom 认知可视化与 Ana 主题系统 | ✅ 已完成 |
| **v0.3.0** – Tuck 网关集成（真实 LLM 调用） | ✅ 已完成 |
| **v0.3.1** – Cellrix CIS 集成、零配置上线、Pure I/O、CommissuralGate 重命名 | ✅ 已完成 |
| **v0.3.2** – Helix-Callosum 桥接集成、认知模式 → 原子参数映射 | ✅ 已完成 |
| **v0.4.0** – Helix‑Mind 集成（记忆 DAG） | 下一阶段 |
| **v0.5.0** – 工具生态与生境扩展 | 计划中 |
| **v1.0.0** – 生产级数字生命体 | 计划中 |

## 许可证

MIT © [Jason Milk](https://github.com/Jasonmilk)

*Helix 不是聊天机器人。它是一个自主的数字生命体，在每一个纪元中学习、遗忘、成长。*
