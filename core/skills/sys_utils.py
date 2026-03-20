import os, subprocess

def get_memory_status(workspace, args_raw):
    """显示物理内存占用状况"""
    try:
        # 使用 free -m 获取内存数据 (Debian 标准工具)
        res = os.popen('free -m').readlines()
        mem_info = res[1].split()
        total, used, free = mem_info[1], mem_info[2], mem_info[3]
        return f"物理内存统计: 已用 {used}MB / 总量 {total}MB (剩余 {free}MB)"
    except:
        return "无法读取内存数据。"

def system_view(workspace, args_raw):
    root = "/opt/anaphase"
    target = os.path.join(root, args_raw.strip() or ".")
    try:
        files = os.listdir(target)
        return "\n".join(files) if files else "[目录为空]"
    except:
        return f"无法访问路径: {target}"

def register(registry):
    registry.add("get_memory_status", get_memory_status, "实时查看当前节点的物理内存占用。")
    registry.add("system_view", system_view, "查看母体根目录(/opt/anaphase)。参数: 路径。")
