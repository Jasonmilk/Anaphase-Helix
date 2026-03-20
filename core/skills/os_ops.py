import os

def list_dir(workspace, args_raw):
    # args_raw 可能是 '.' 或者 'src/main'
    target = os.path.join(workspace, args_raw.strip().strip("'\"") or ".")
    if not os.path.exists(target): 
        return f"路径不存在: {target}"
    files = os.listdir(target)
    return "\n".join(files) if files else "[该目录为空]"

def read_file(workspace, args_raw):
    target = os.path.join(workspace, args_raw.strip().strip("'\""))
    if not os.path.exists(target): return "文件不存在。"
    try:
        with open(target, 'r', encoding='utf-8') as f: 
            return f.read()[:2000] # 物理截断防止超载
    except Exception as e: 
        return str(e)

def register(registry):
    """技能注册入口"""
    registry.add("list_dir", list_dir, "列出指定目录下的文件。参数: 相对路径(如 '.')。")
    registry.add("read_file", read_file, "读取指定文件内容。参数: 文件相对路径。")
