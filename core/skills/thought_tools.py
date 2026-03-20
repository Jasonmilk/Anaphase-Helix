import os

def scratchpad(workspace, args_raw):
    """读写当前任务的记事本。参数: read 或 write|内容"""
    path = os.path.join(workspace, "scratchpad.md")
    if args_raw.startswith("write|"):
        content = args_raw.split("|", 1)[1]
        with open(path, "w", encoding="utf-8") as f: f.write(content)
        return "记事本已更新。"
    else:
        if not os.path.exists(path): return "记事本为空。请先规划你的任务。"
        with open(path, "r", encoding="utf-8") as f: return f.read()

def update_subjective_thought(workspace, args_raw):
    """将进化感悟存入主观思想库。参数: 感悟内容"""
    path = "/opt/anaphase/global_mind/evolution_notes.md"
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "a", encoding="utf-8") as f:
        f.write(f"\n- [进化记录] {args_raw.strip()}")
    return "思想已固化至长久进化笔记。"

def register(registry):
    registry.add("scratchpad", scratchpad, "任务备忘录。参数: read 或 write|内容。")
    registry.add("update_subjective_thought", update_subjective_thought, "提炼长期感悟。")
