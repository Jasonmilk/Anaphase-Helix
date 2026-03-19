import os
import json
import time
import re
from core.config import settings

class Chronicler:
    def __init__(self):
        self.report_path = os.path.join(settings.ANAPHASE_ROOT, "evolution_briefing.md")

    def write_briefing(self, status, task, metrics, last_words=None):
        timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
        
        # 处理 Runtime 格式
        rt = metrics.get('runtime', 'N/A')
        rt_str = f"{rt:.4f} s" if isinstance(rt, (int, float)) else str(rt)
        
        # 确定笔记内容 (Note)
        if status == "SUCCESS":
            status_icon = "🟢 进化成功"
            default_note = "物种表现出极高的工具理性，成功将逻辑固化为物理资产。"
        else:
            status_icon = "🔴 物理淘汰"
            default_note = "逻辑突变失败，未通过环境筛选。"
            
        note = last_words if last_words else default_note
        # 移除可能的 Markdown 标签或换行，保持 Commit 消息整洁
        clean_note = re.sub(r'[\n\r#*`]', ' ', note).strip()

        entry = f"""
## 🧬 演化脉冲 [{timestamp}]
- **状态**: {status_icon}
- **任务**: `{task}`
- **代谢 (Token)**: {metrics.get('tokens', 'N/A')} t (最优: {metrics.get('best_tokens')} t)
- **物理 (Runtime)**: {rt_str} (最优: {metrics.get('best_runtime'):.4f} s)
- **遗产**: 累计固化 {metrics.get('total_evolutions', 0)} 个工具

### 💡 观察员笔记
> {note}

---
"""
        # 写入文件逻辑 (保持 Markdown 记录)
        header = "# 📓 Anaphase-Helix 演化观察日志\n\n"
        content = header + entry + (open(self.report_path).read().replace(header, "") if os.path.exists(self.report_path) else "")
        with open(self.report_path, "w", encoding="utf-8") as f: f.write(content)
        
        # 返回给 Git Commit 的摘要 (限制长度)
        return f"{status_icon} | {clean_note[:60]}..."

chronicler = Chronicler()
