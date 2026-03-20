import os

class MemoryCortex:
    def __init__(self, workspace):
        self.workspace = workspace
        self.notes_path = "/opt/anaphase/global_mind/evolution_notes.md"

    def get_distilled_thoughts(self):
        """物理层面的认知蒸馏：保持高浓度，限制体积"""
        if not os.path.exists(self.notes_path):
            return "初生状态：暂无进化思想。"

        with open(self.notes_path, "r", encoding="utf-8") as f:
            lines = f.readlines()

        # 如果笔记多于 15 条，进行“物理归并”
        if len(lines) > 15:
            # 保留最核心的前 3 条（初衷）和最后 7 条（最新进展）
            distilled = lines[:3] + ["\n...[历史演进细节已压缩为潜意识]...\n"] + lines[-7:]
            return "".join(distilled)
        
        return "".join(lines)

    def wake_up(self):
        path = os.path.join(self.workspace, "session_notes.json")
        if os.path.exists(path):
            try:
                with open(path, 'r') as f: return f.read()[:500]
            except: pass
        return "无任务内短期记忆。"
