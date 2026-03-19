import os, json, time, re
from core.config import settings
from core.tuck_gateway import tuck_gw

class MemoryCortex:
    def __init__(self, workspace_path):
        self.notes_path = os.path.join(workspace_path, "session_notes.json")

    def wake_up(self):
        if os.path.exists(self.notes_path):
            try:
                with open(self.notes_path, 'r') as f:
                    data = json.load(f)
                    findings = data.get("findings", "")
                    # 【核心拦截】如果记忆中全是数字或数学方括号，判定为污染，强行重置
                    if re.match(r'^[\[\d,.\-\s\]]+$', findings):
                        print("[Memory] 检测到恶意数学噪音，已自动清洗记忆。")
                        return "前世尝试失败，陷入了无意义的数学循环。请重新从 search_code 开始探测。"
                    return findings
            except: pass
        return "这是第一次探索，请先使用 toolkit.list_dir('.') 了解目录结构。"

    def dehydrate(self, history, current_findings):
        """强制 Refiner 进入『工程记录』模式"""
        print("[Memory] 史官正在脱水历史...")
        
        # 只取对话中关键的物理反馈
        raw_context = ""
        for h in history:
            if "【物理反馈】" in h['content']:
                raw_context += h['content'][:200] + "\n"

        prompt = [{"role": "user", "content": f"""
你是一名严谨的实验室记录员。请总结当前的 Bug 修复进度。
【已知事实】: （记录已发现的文件路径和代码行号）
【当前卡点】: （记录报错信息或找不到的文件）
【下一步建议】: （给出具体的 toolkit 指令建议）

禁止输出任何数学计算结果！禁止输出 [1,2,3] 这种数组！只写工程简报。
对话背景：
{raw_context}
"""}]
        
        try:
            res = tuck_gw.invoke_helix(prompt, settings.TUCK_PERSONA_WORKER, settings.MODEL_REFINER)
            summary = res['content'].strip()
            
            # 基础过滤逻辑
            if len(summary) < 5 or summary.startswith("["):
                summary = "进度总结失败，请根据 Issue 重新开始。"

            with open(self.notes_path, "w", encoding="utf-8") as f:
                json.dump({"last_update": time.time(), "findings": summary}, f, ensure_ascii=False, indent=2)
            return summary
        except:
            return "记忆保存异常。"
