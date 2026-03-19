import os, time
from core.config import settings

class Chronicler:
    def __init__(self):
        self.report_path = os.path.join(settings.ANAPHASE_ROOT, "evolution_briefing.md")

    def write_briefing(self, status, task, metrics):
        timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
        preview = metrics.get('preview', '无代码修改预览')
        
        entry = f"""
## 🧬 协同演化报告 [{timestamp}]
- **状态**: {"🟢 修复成功" if status == "SUCCESS" else "🔴 失败"}
- **任务**: `{task[:100]}`
- **能耗**: {metrics.get('tokens', 0)} tokens
- **凭证**: `{metrics.get('path', 'N/A')}`

### 🔍 关键修改预览 (Git Diff Top 10)
```diff
{preview}

---
"""
        header = "# 📓 Anaphase-Helix 演化观察日志\n\n"
        old = open(self.report_path).read().replace(header, "") if os.path.exists(self.report_path) else ""
        with open(self.report_path, "w") as f: f.write(header + entry + old)
        return f"{status} | {task[:30]}..."

chronicler = Chronicler()
