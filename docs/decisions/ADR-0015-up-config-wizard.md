# ADR-0015: 配置向导（up 菜单选项 4）——LLM 引导输入

- **状态**: Accepted
- **日期**: 2026-09-06
- **关联**: ADR-0013（交互菜单）、ADR-0014（Web 面板）、用户约束（2026-09-06：
  **"Anaphase 有没有引导我输入 api key?"**）
- **仓库**: Anaphase（src/bin/up.rs）

## 1. 背景

up 交互菜单（ADR-0013）只有"配置说明"（只读打印字段名），用户仍需手动
编辑 config.toml 输入 reasoning_endpoint / model / api_key。用户明确询问
"有没有引导我输入 api key"——没有。本 ADR 把配置也做成选择题
（"一个命令之后不再出现命令"哲学的延伸）：菜单选项 4 = 配置 LLM 向导。

## 2. 决策

### D1: 菜单选项 4 = 配置 LLM（引导输入）

交互一问一答（Enter = 跳过保持现值）：
1. base_url（OpenAI 兼容端点）
2. model
3. api_key（**输入不回显**）

只改三个字段，其余 config.toml 字节级保留（行级替换，纯函数可测）。

### D2: api_key 不回显——stty -echo 包裹，零新依赖

`read_line_hidden()`：`stty -echo` → 读行 → `stty echo`（macOS/Linux
通用命令，非新 crate）。实测 pty 捕获输出**不含** api key 明文。

### D3: 写盘前备份

原文件 → `config.toml.bak`（`with_extension("toml.bak")`）→ 写回。
覆盖是向导的显式意图，.bak 保证可恢复；用户按需删除。

### D4: 零硬编码与确定性

- 字段名来自现有 config.toml 键（`reasoning_endpoint`/`reasoning_model`/
  `reasoning_api_key`），不新造配置
- `apply_reasoning_updates` 纯函数：行级替换、空输入跳过、值转义
  （`escape_toml` 处理 `"` 与 `\`）；字段行缺失 → 明确 warn（不静默）
- 全空输入 → "无变更" 不写盘（不产生无谓 .bak）

## 3. 验证（已通过）

1. `cargo test` Anaphase **140 passed**（135 + 5：parse_choice "4"=Configure
   + apply_updates 4 用例，无回归）
2. **pty 交互实测**：菜单 → 4 → 输入 base_url/model/api_key → "已写入
   config.toml（原文件备份: config.toml.bak）" → 三字段真实替换 → 还原
   后回到空值
3. **回显验证**：pty 捕获输出 `api key leaked: False`
4. 备份/还原往返一致（config.toml 测试后恢复原始空值）

## 4. 后果

**正面**：LLM 配置零命令（选择题完成）；api key 不回显（肩窥防护）；
写盘可恢复；字段缺失诚实警告。

**代价/缺口**：
- 仅写 reasoning 三字段——mode/端口等仍走配置说明（后续按需扩向导）
- api key 明文落盘（config.toml）——与常见工具一致（`.env` 同权）；
  系统级密钥托管（Keychain/Tuck 保险柜）属 Tuck 深度集成（D'-2）范围
- `stty` 依赖系统命令；非 macOS/Linux 环境需替换实现（当前平台已覆盖）
