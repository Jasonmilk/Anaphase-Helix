# Anaphase-Helix v0.2.0

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)
[![EN](https://img.shields.io/badge/English-README-blue)](./README.md)

**Anaphase-Helix** 是 Helix 生态的执行编排中枢——一个自进化的数字生命体。它协调感知（Tentacle）、记忆（Mind）与推理，通过状态图驱动的 Agent Loop 完成复杂任务。

> **当前状态**：v0.2.0 – 全模块骨架与 Mock 模式集成已完成。所有脑区（杏仁核、前额叶、突触、胼胝体）已贯通，Agent Loop 可在 Mock 模式下端到端执行。已为 Tuck 网关集成做好准备。

## 🧠 核心哲学

- **编排优先，拒造实体** – 核心只做调度；实质性工作委托给外部 CLI 工具或微服务。
- **契约至上** – 所有跨模块通信使用严格的 Pydantic DTO。
- **DAG 化一切** – 知识、任务、工具、记忆均建模为有向无环图。
- **引导而非阻断** – Anaphase 以劝导为主；Tuck 作为最后防线执行遏制。
- **硅基代谢** – 主动管理 Token 预算与认知负荷；Agent 在疲劳时进入“睡眠”。

## 🚀 快速开始

### 环境要求
- Python 3.12+
- [Tuck 网关](https://github.com/Jasonmilk/Tuck)（可选，用于真实 LLM 调用）

### 安装

```bash
git clone https://github.com/Jasonmilk/Anaphase-Helix.git
cd Anaphase-Helix
git checkout V5

python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -e ".[dev]"
```

### 配置

复制环境变量模板并编辑：

```bash
cp .env.example .env
```

**Mock 模式**（无需 LLM）：
```ini
ANA_MOCK_MODE=true
```

**真实 LLM 调用**（通过 Tuck）：
```ini
ANA_MOCK_MODE=false
TUCK_ENDPOINT=http://localhost:8686
TUCK_API_KEY=你的_API_密钥
```

### 执行首个任务

```bash
ana run "人生的意义是什么？"
```

在 Mock 模式下，你将看到完整的认知循环追踪日志（JSON 格式），并以 Mock 推理草稿结束。Agent 将依次经历七个状态：`perceive → assess_priority → plan → execute → reflect → consolidate → sleep`。

## 📁 项目结构

```
Anaphase-Helix/
├── ana/
│   ├── cli/                 # CLI 入口（ana run/trace/stats）
│   ├── core/                # 脑区模块
│   │   ├── agent_loop.py    # 状态图驱动主循环
│   │   ├── amygdala.py      # 优先级与情感评估
│   │   ├── prefrontal.py    # 推理与规划（LLM 调用）
│   │   ├── synapse.py       # 工具执行（CLI 安全沙箱）
│   │   ├── corpus_callosum.py # 意图-执行对齐校验器
│   │   └── model_router.py  # 基于优先级选择模型
│   ├── schemas/             # Pydantic DTO 契约
│   ├── common/              # 配置、日志、追踪、重试
│   └── registry/            # 工具注册表（加载 config/tools.yaml）
├── config/                  # 基因锁、工具清单、生境模板
├── knowledge_base/          # L1 自画像（self.md）
├── tests/                   # 单元测试（13 个已通过）
├── docs/                    # 白皮书与工程手册
├── .env.example
├── pyproject.toml
└── README.md
```

## 🧪 测试

运行完整测试套件：

```bash
pytest -v
```

Mock 模式下 13 个测试全部通过，覆盖所有脑区模块及 Agent Loop 集成。

## 📖 文档

- [Helix 生态统一白皮书](docs/WHITEPAPER.md) – “是什么”与“为什么”。
- [Anaphase-Helix 工程手册](docs/ENGINEERING.md) – “怎么做”：项目结构、DTO、AI Coder 铁律及工作流。

## 🤝 AI Coder 协作

本项目遵循严格的 **AI Coder 铁律清单**（参见工程手册 §7.1），确保 LLM 生成的代码保持一致性、可测试性，并与 Helix 哲学对齐。每个模块均采用测试先行开发，Mock 实现验证通过后再接入真实后端。

## 📌 路线图

| 里程碑 | 状态 |
|:---|:---|
| **v0.1.0** – 物理骨架（目录、DTO、CLI） | ✅ 已完成 |
| **v0.2.0** – 全模块 Mock 集成与端到端验证 | ✅ 已完成 |
| **v0.3.0** – Tuck 网关集成（真实 LLM 调用） | 🚧 下一阶段 |
| **v0.4.0** – Helix‑Mind 集成（记忆 DAG） | 📅 计划中 |
| **v0.5.0** – 工具生态与生境扩展 | 📅 计划中 |
| **v1.0.0** – 生产级数字生命体 | 📅 计划中 |

## 📄 许可证

MIT © [Jason Milk](https://github.com/Jasonmilk)

---

*Helix 不是聊天机器人。它是一个自主的数字生命体，在每一个纪元中学习、遗忘、成长。*
