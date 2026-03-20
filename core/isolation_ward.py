import os, subprocess, resource, shutil

class IsolationWard:
    def __init__(self, issue_number: str, repo_url: str):
        self.issue_number = str(issue_number)
        if "api.github.com/repos/" in repo_url:
            self.repo_url = repo_url.replace("api.github.com/repos/", "github.com/") + ".git"
        else:
            self.repo_url = repo_url
        self.workspace = f"/opt/anaphase/workspaces/issue_{self.issue_number}"

    def _set_strict_limits(self):
        resource.setrlimit(resource.RLIMIT_AS, (500 * 1024 * 1024, 500 * 1024 * 1024))
        resource.setrlimit(resource.RLIMIT_CPU, (60, 60))
        resource.setrlimit(resource.RLIMIT_NPROC, (100, 100))

    def clone_repository(self):
        """核心修复：如果目录已存在，绝不重新 Clone，保护记忆文件"""
        if os.path.exists(self.workspace):
            print(f"[隔离舱] 检测到现存沙盒，保留物理记忆，跳过 Clone。")
            return True
        
        print(f"[隔离舱] 正在创建初次实验环境: {self.repo_url}")
        try:
            subprocess.run(["git", "clone", "--depth", "1", self.repo_url, self.workspace],
                check=True, capture_output=True, text=True, timeout=120)
            return True
        except Exception as e:
            print(f"[隔离舱 Error] 初次克隆失败: {e}")
            return False

    def run_safe_command(self, cmd_list):
        try:
            process = subprocess.Popen(cmd_list, cwd=self.workspace,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                preexec_fn=self._set_strict_limits, text=True)
            stdout, stderr = process.communicate(timeout=65)
            return (process.returncode == 0, stdout.strip() if process.returncode == 0 else stderr.strip())
        except Exception as e: return (False, str(e))
