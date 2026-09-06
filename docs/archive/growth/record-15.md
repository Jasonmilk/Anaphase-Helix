## 记录 15：G-7 配置向导（2026-09-06）

**变异类型**：LLM 引导输入——用户（2026-09-06）"Anaphase 有没有引导我输入 api key?"

- ADR-0015：up 菜单选项 4 = 配置 LLM（base_url/model/api_key 一问一答，Enter 跳过保持现值）
- api_key 输入不回显（stty -echo 包裹，零新依赖；pty 实测捕获输出无 key 明文）
- 写盘前备份 config.toml.bak；行级替换其余字节保留；空输入不写盘；字段缺失诚实 warn
- 实测：菜单 → 4 → 三字段写入 → 备份存在 → 还原往返一致
- 140 tests 全绿（135 + 5）；下一步 G2 Web 优化
