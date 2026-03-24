import os, subprocess

def list_dir(workspace, args_raw):
    target = os.path.join(workspace, args_raw.strip().strip("'\"") or ".")
    if not os.path.exists(target): return f"路径不存在: {target}"
    try:
        items = os.listdir(target)
        report =[]
        for item in items:
            path = os.path.join(target, item)
            size = os.path.getsize(path) if os.path.isfile(path) else "[DIR]"
            report.append(f"{item} ({size})")
        return "\n".join(report) if report else "[目录为空]"
    except Exception as e: return str(e)

def read_file(workspace, args_raw):
    target = os.path.join(workspace, args_raw.strip().strip("'\""))
    if not os.path.exists(target): return "文件不存在。"
    size = os.path.getsize(target)
    if size > 10240: return f"[警告] 文件为 {size} 字节。为防 OOM，请使用 shell_cmd 通过 grep 或 head 命令查阅。"
    try:
        with open(target, 'r', encoding='utf-8') as f: return f.read()
    except Exception as e: return str(e)

def shell_cmd(workspace, args_raw):
    """
    执行原生 Linux Shell 命令。这是编译 Rust (cargo build) 的核心工具。
    """
    cmd = args_raw.strip().strip("'\"")
    print(f"🖥️  底层执行: {cmd}")
    try:
        # 设置 timeout 为 60s，防止 cargo build 卡死进程
        res = subprocess.run(cmd, shell=True, cwd=workspace, capture_output=True, text=True, timeout=60)
        output = res.stdout if res.returncode == 0 else res.stderr
        # 物理截断防止炸显存
        return f"[Exit Code: {res.returncode}] {output[:1500]}"
    except subprocess.TimeoutExpired:
        return "[错误] Shell 命令执行超时 (60s)。若在编译 Rust，请确认没有引发死锁。"
    except Exception as e:
        return f"[执行异常] {str(e)}"

def register(registry):
    registry.add("list_dir", list_dir, "列出目录及大小。参数: 路径。")
    registry.add("read_file", read_file, "读取文件。参数: 路径。")
    registry.add("shell_cmd", shell_cmd, "执行Linux shell命令(如 cargo build/cargo new)。参数: 命令字符串。")
