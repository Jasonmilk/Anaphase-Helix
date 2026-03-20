import subprocess
import os

def commit_and_push(workspace, args_raw):
    """将工作区的改变同步到 GitHub"""
    msg = args_raw.strip().strip("'\"")
    if not msg: return "[错误] 必须提供 Commit 信息。"
    
    # 根据是在修改全局灵魂，还是在修局部 Bug，决定 Git 目录
    target_dir = "/opt/anaphase" if "soud.md" in msg or "进化" in msg else workspace
    
    try:
        # 依次执行 add, commit, push
        subprocess.run(["git", "add", "."], cwd=target_dir, check=True, capture_output=True)
        # 允许 commit 为空（如果文件没变化）
        subprocess.run(["git", "commit", "-m", f"🤖 Helix 进化记录: {msg}"], cwd=target_dir, capture_output=True)
        push_res = subprocess.run(["git", "push"], cwd=target_dir, capture_output=True, text=True)
        
        if push_res.returncode == 0:
            return f"[Git] 成功！您的成就已作为 Commit 推送至 GitHub: {msg}"
        else:
            return f"[Git 警告] 推送失败，可能是权限或无新更改: {push_res.stderr[:200]}"
    except Exception as e:
        return f"[Git 异常] {str(e)}"

def register(registry):
    registry.add("commit_and_push", commit_and_push, "当你完成代码修改或更新思想后，用此工具将成果推送到GitHub。参数: 你的总结(一句话)。")
