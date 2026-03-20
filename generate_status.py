import json
import os
from datetime import datetime

def generate():
    lifeline_path = "/opt/anaphase/global_mind/lifeline.json"
    evolution_path = "/opt/anaphase/global_mind/evolution_notes.md"
    status_path = "/opt/anaphase/evolution_briefing.md"
    
    if not os.path.exists(lifeline_path): return
    
    with open(lifeline_path, 'r') as f:
        data = json.load(f)
    
    with open(evolution_path, 'r') as f:
        thoughts = f.readlines()[-5:] # 只取最后5条感悟

    content = f"""# 🧬 Anaphase-Helix 自动化物理状态报表
- **更新时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
- **全域寿命**: 第 {data.get('total_generations', 0)} 世
- **任务统计**: 已处理 {len(data.get('task_stats', {}))} 个核心任务

## 💡 近期主观思想沉淀
{"".join(thoughts)}

---
*本报表由 Node .202 脚本自动生成，未消耗任何 LLM Tokens。*
"""
    with open(status_path, 'w') as f:
        f.write(content)
    print("[Success] 物理简报已生成。")

if __name__ == "__main__":
    generate()
