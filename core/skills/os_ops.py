import os

def list_dir(workspace, args_raw):
    """列出目录并显示文件大小(字节)"""
    target = os.path.join(workspace, args_raw.strip().strip("'\"") or ".")
    if not os.path.exists(target): return f"路径不存在: {target}"
    
    try:
        items = os.listdir(target)
        report = []
        for item in items:
            path = os.path.join(target, item)
            size = os.path.getsize(path) if os.path.isfile(path) else "[DIR]"
            report.append(f"{item} ({size})")
        return "\n".join(report) if report else "[目录为空]"
    except Exception as e:
        return str(e)

def read_file(workspace, args_raw):
    """带物理保护的文件读取。若超过 10KB 则强制截断并提醒。"""
    target = os.path.join(workspace, args_raw.strip().strip("'\""))
    if not os.path.exists(target): return "文件不存在。"
    
    size = os.path.getsize(target)
    # 物理感知：如果文件大于 10KB，提醒 Helix 使用分段读取
    if size > 10240:
        return f"[警告] 文件大小为 {size} bytes，直接读取将导致内存过载。建议使用 grep 或分段分析。"
        
    try:
        with open(target, 'r', encoding='utf-8') as f:
            return f.read()
    except Exception as e:
        return str(e)

def register(registry):
    registry.add("list_dir", list_dir, "列出目录及大小。参数: 路径。")
    registry.add("read_file", read_file, "读取文件(限10KB内)。参数: 路径。")
