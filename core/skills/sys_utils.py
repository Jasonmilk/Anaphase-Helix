import os, subprocess

def system_view(workspace, args_raw):
    """查看母体系统根目录结构。参数: 目录名(如 'core')"""
    root = "/opt/anaphase"
    target = os.path.join(root, args_raw.strip() or ".")
    try:
        return "\n".join(os.listdir(target))
    except:
        return f"无法访问母体路径: {target}"

def use_scratchpad(workspace, args_raw):
    """读写任务备忘录。参数格式: read 或 write|内容"""
    path = os.path.join(workspace, "scratchpad.md")
    if args_raw.startswith("write|"):
        content = args_raw.split("|", 1)[1]
        with open(path, "w") as f: f.write(content)
        return "备忘录已更新。"
    else:
        if not os.path.exists(path): return "备忘录为空。"
        with open(path, "r") as f: return f.read()

def update_subjective_thought(workspace, args_raw):
    """提炼思想。仅在轮回结束时调用。"""
    path = "/opt/anaphase/global_mind/evolution_notes.md"
    with open(path, "a") as f:
        f.write(f"\n- [进化记录] {args_raw.strip()}")
    return "思想已固化。"

def register(registry):
    registry.add("system_view", system_view, "查看母体根目录(/opt/anaphase)。参数: 路径。")
    registry.add("use_scratchpad", use_scratchpad, "操作工作白板。参数: read 或 write|内容。")
    registry.add("update_subjective_thought", update_subjective_thought, "提炼并存储长期的进化感悟。")
