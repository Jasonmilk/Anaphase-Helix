import json
import os
from datetime import datetime

class Lifeline:
    def __init__(self):
        self.path = "/opt/anaphase/global_mind/lifeline.json"
        os.makedirs(os.path.dirname(self.path), exist_ok=True)
        self.data = self._load()

    def _load(self):
        # 科学定义的标准数据结构
        template = {"total_generations": 0, "tasks_completed": 0, "task_stats": {}}
        
        if os.path.exists(self.path):
            try:
                with open(self.path, 'r', encoding='utf-8') as f:
                    content = json.load(f)
                    # 【核心修复】：将旧数据合并到模板中，确保所有新键都存在
                    template.update(content)
                    # 兼容性转换：如果旧版叫 total_blinks，自动转为 total_generations
                    if "total_blinks" in template:
                        template["total_generations"] = max(template["total_generations"], template.pop("total_blinks"))
                    return template
            except Exception as e:
                print(f"[Lifeline] 档案读取异常，正在初始化: {e}")
        return template

    def _save(self):
        with open(self.path, 'w', encoding='utf-8') as f:
            json.dump(self.data, f, indent=4, ensure_ascii=False)

    def heartbeat(self, task_id):
        """记录全域寿命与任务周期"""
        # 再次确保键存在（双重保险）
        if "total_generations" not in self.data:
            self.data["total_generations"] = 0
            
        self.data["total_generations"] += 1
        t_id = str(task_id)
        
        if t_id not in self.data["task_stats"]:
            self.data["task_stats"][t_id] = {
                "cycles": 0, 
                "start_time": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            }
        
        self.data["task_stats"][t_id]["cycles"] += 1
        self.data["task_stats"][t_id]["last_active"] = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        self._save()
        return self.data["total_generations"], self.data["task_stats"][t_id]["cycles"]
