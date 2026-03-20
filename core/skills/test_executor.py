import subprocess
import os

def run_python_task(workspace, args_raw):
    """在沙盒内运行指定的Python脚本并捕获输出"""
    script_path = os.path.join(workspace, args_raw.strip().strip("'\""))
    if not os.path.exists(script_path): return f"[错误] 脚本 {args_raw} 不存在。"
    
    try:
        # 限制运行时间为 30s，防止死循环耗尽 ARM 资源
        res = subprocess.run(
            ["python3", script_path], 
            cwd=workspace, 
            capture_output=True, 
            text=True, 
            timeout=30
        )
        output = res.stdout if res.returncode == 0 else res.stderr
        return f"[执行结果 (Code {res.returncode})]:\n{output[:1000]}"
    except subprocess.TimeoutExpired:
        return "[错误] 脚本运行超时(30s)，疑似死循环。"
    except Exception as e:
        return f"[执行异常] {str(e)}"

def register(registry):
    registry.add("run_python_task", run_python_task, "在当前工作区运行Python脚本并获取反馈。参数: 脚本路径。")
