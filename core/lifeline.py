import json
import os
from datetime import datetime

class Lifeline:
    def __init__(self):
        self.path = "/opt/anaphase/global_mind/lifeline.json"
        os.makedirs(os.path.dirname(self.path), exist_ok=True)
        self.data = self._load()

    def _load(self):
        if os.path.exists(self.path):
            try:
                with open(self.path, 'r') as f: return json.load(f)
            except: pass
        return {"total_generations": 0, "tasks_completed": 0, "task_stats": {}}

    def _save(self):
        with open(self.path, 'w') as f:
            json.dump(self.data, f, indent=4)

    def heartbeat(self, task_id):
        """核心心跳：增加全域寿命并追踪当前任务周期"""
        self.data["total_generations"] += 1
        t_id = str(task_id)
        if t_id not in self.data["task_stats"]:
            self.data["task_stats"][t_id] = {"cycles": 0, "start_time": datetime.now().isoformat()}
        
        self.data["task_stats"][t_id]["cycles"] += 1
        self.data["task_stats"][t_id]["last_active"] = datetime.now().isoformat()
        self._save()
        return self.data["total_generations"], self.data["task_stats"][t_id]["cycles"]

    def get_age_report(self, task_id):
        t_id = str(task_id)
        total = self.data["total_generations"]
        cycle = self.data["task_stats"].get(t_id, {}).get("cycles", 0)
        return f"Helix 第 {total} 世 | 当前任务接力第 {cycle} 轮"
