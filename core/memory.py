import os, json, time, re
from core.config import settings
from core.tuck_gateway import tuck_gw

class MemoryCortex:
    def __init__(self, workspace_path):
        self.notes_path = os.path.join(workspace_path, "session_notes.json")

    def wake_up(self):
        """苏醒协议：只提取最干的货"""
        if os.path.exists(self.notes_path):
            try:
                with open(self.notes_path, 'r') as f:
                    data = json.load(f)
                    return data.get("kernel", "初次启动。")
            except: pass
        return "无前世记忆。"

    def dehydrate(self, history, target_title):
        """记忆脱水：将万字长文压榨为 200 字以内的『认知内核』"""
        print("[Memory] 正在执行认知内核压榨...")
        
        # 只提取物理反馈的关键行
        evidence = ""
        for h in history:
            if "【物理反馈】" in h['content']:
                lines = h['content'].split('\n')
                evidence += "\n".join(lines[:3]) + "\n" # 只取反馈的前3行核心信息

        prompt = [{"role": "user", "content": f"""
你是一名资深史官。请将以下调试历史压榨为极简『认知内核』。
必须严格遵守格式：
1. 已锁定文件: [路径]
2. 核心逻辑: [一句话总结]
3. 当前精确位置: [文件名+行号]
4. 绝对禁忌: [前世踩过的坑]

任务目标: {target_title}
历史证据:
{evidence}
"""}]
        
        try:
            res = tuck_gw.invoke_helix(prompt, settings.TUCK_PERSONA_WORKER, settings.MODEL_REFINER)
            kernel = res['content'].strip()
            
            # 存入物理磁盘
            with open(self.notes_path, "w", encoding="utf-8") as f:
                json.dump({"last_update": time.time(), "kernel": kernel}, f, ensure_ascii=False, indent=2)
            return kernel
        except:
            return "脱水异常。"
