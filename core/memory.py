import os

class MemoryCortex:
    def __init__(self, workspace):
        self.workspace = workspace
        self.notes_path = "/opt/anaphase/global_mind/evolution_notes.md"

    def get_distilled_thoughts(self):
        """严格节流：物理截断至 300 字符内，绝不越界"""
        if not os.path.exists(self.notes_path):
            return "初生状态：暂无进化思想。"
        
        with open(self.notes_path, "r", encoding="utf-8") as f:
            content = f.read().strip()
            
        if len(content) > 300:
            # 仅保留最新的思想结晶
            return "...[早期思想已归档]...\n" + content[-280:]
        return content
