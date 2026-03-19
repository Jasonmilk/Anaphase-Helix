import os, json, time
from core.config import settings
from core.tuck_gateway import tuck_gw

class MemoryCortex:
    """记忆皮层：处理认知快照、脱水与传承"""
    def __init__(self, workspace_path):
        self.notes_path = os.path.join(workspace_path, "session_notes.json")

    def wake_up(self):
        """强制苏醒协议：提取前世遗留在沙盒中的认知精华"""
        if os.path.exists(self.notes_path):
            with open(self.notes_path, 'r') as f:
                data = json.load(f)
                return data.get("findings", "尚未发现有效线索。")
        return "这是此任务的第一次探索，请从零开始。"

    def dehydrate(self, history, current_findings):
        """执行认知脱水：利用 2B 模型将繁琐的历史压缩为简报"""
        print("[Memory] 正在执行认知脱水，提取核心资产...")
        history_text = "\n".join([f"{h['role']}: {h['content'][:300]}" for h in history[-5:]])
        
        prompt = [{"role": "user", "content": f"请以首席工程师的口吻，总结目前修复Bug的进展、已确认的文件位置、及下一步具体计划。\n历史线索:\n{history_text}\n当前直觉: {current_findings}"}]
        
        try:
            res = tuck_gw.invoke_helix(prompt, settings.TUCK_PERSONA_WORKER, settings.MODEL_REFINER)
            summary = res['content'] if res['tokens_used'] > 0 else "总结异常"
            
            with open(self.notes_path, "w", encoding="utf-8") as f:
                json.dump({"last_update": time.time(), "findings": summary}, f, ensure_ascii=False, indent=2)
            return summary
        except Exception as e:
            return f"脱水失败: {e}"

