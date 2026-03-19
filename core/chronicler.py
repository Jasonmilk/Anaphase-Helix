import os
import json
import time
from core.config import settings

class Chronicler:
    def __init__(self):
        self.report_path = os.path.join(settings.ANAPHASE_ROOT, "evolution_briefing.md")

    def write_briefing(self, status, task, metrics, last_words="N/A"):
        timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
        
        # 安全处理 Runtime 格式
        rt = metrics.get('runtime', 'N/A')
        rt_str = f"{rt:.4f} s" if isinstance(rt, (int, float)) else str(rt)
        
        # 安全处理 Token 格式
        tk = metrics.get('tokens', 'N/A')
        
        status_icon = "🟢 进化成功" if status == "SUCCESS" else "🔴 物理淘汰"
        
        entry = f"""
## 🧬 演化脉冲 [{timestamp}]
- **状态**: {status_icon}
- **任务**: `{task}`
- **代谢 (Token)**: {tk} t (最优: {metrics.get('best_tokens')} t)
- **物理 (Runtime)**: {rt_str} (最优: {metrics.get('best_runtime'):.4f} s)
- **遗产**: 累计固化 {metrics.get('total_tools', 0)} 个工具

### 💡 观察员笔记
> {last_words if status == "FAILURE" else "物种表现出极高的工具理性，成功将逻辑固化为物理资产。"}

---
"""
        header = "# 📓 Anaphase-Helix 演化观察日志\n\n"
        if not os.path.exists(self.report_path):
            content = header + entry
        else:
            with open(self.report_path, "r", encoding="utf-8") as f:
                old_content = f.read()
            content = header + entry + old_content.replace(header, "")

        with open(self.report_path, "w", encoding="utf-8") as f:
            f.write(content)
        
        # 返回这行笔记，供 Git Commit 使用
        return f"{status_icon} | {last_words[:50]}"

chronicler = Chronicler()
