import os, json, time
from core.config import settings
from core.tuck_gateway import tuck_gw

class MemoryCortex:
    def __init__(self, workspace_path):
        self.notes_path = os.path.join(workspace_path, "session_notes.json")

    def wake_up(self):
        if os.path.exists(self.notes_path):
            try:
                with open(self.notes_path, 'r') as f:
                    return json.load(f).get("auditor_report", "初始状态。")
            except: pass
        return "无历史迭代记录。"

    def dehydrate(self, history):
        # 核心逻辑：仅在有物理反馈时生成迭代记录
        evidence = "\n".join([h['content'] for h in history if "【物理反馈】" in h['content']])
        if not evidence.strip():
            return "本轮无实质操作进展。"

        print("[Memory] 生成迭代合规记录...")
        # 去拟人化的审计员提示词
        auditor_sys = """你是迭代记录员。仅从物理反馈中提取以下信息，严禁分析或建议：
1. 已探索的文件/路径
2. 出现的报错信息
3. 已执行的工具操作
未提及的项填“无”。"""
        try:
            res = tuck_gw.invoke_helix(
                [{"role":"system","content":auditor_sys},{"role":"user","content":evidence[-2000:]}], 
                settings.TUCK_PERSONA_AUDITOR, 
                settings.MODEL_REFINER
            )
            report = res['content'].strip()
            with open(self.notes_path, "w") as f:
                json.dump({"last_update": time.time(), "auditor_report": report}, f)
            return report
        except Exception as e:
            return f"迭代记录生成失败: {str(e)}"
