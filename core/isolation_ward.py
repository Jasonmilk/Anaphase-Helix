import os
import subprocess
import resource
import shutil
from core.config import settings

class IsolationWard:
    """绝对物理沙盒：负责安全克隆、硬隔离执行与环境清理"""
    
    def __init__(self, issue_number: str, repo_url: str):
        self.issue_number = str(issue_number)
        
        # 【自动纠偏】将 GitHub API URL 转换为可 Clone 的 HTTPS URL
        if "api.github.com/repos/" in repo_url:
            self.repo_url = repo_url.replace("api.github.com/repos/", "github.com/") + ".git"
        else:
            self.repo_url = repo_url
            
        self.workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{self.issue_number}")

    def _set_strict_limits(self):
        """【核心红线】在子进程启动前，由 Linux 内核强制注入物理限制"""
        # 1. 限制最大内存 (500MB，防止 OOM 拖垮宿主机)
        resource.setrlimit(resource.RLIMIT_AS, (500 * 1024 * 1024, 500 * 1024 * 1024))
        # 2. 限制 CPU 时间 (60秒，防止死循环干烧 ARM 算力)
        resource.setrlimit(resource.RLIMIT_CPU, (60, 60))
        # 3. 限制最大写文件大小 (50MB，防止恶意写满磁盘)
        resource.setrlimit(resource.RLIMIT_FSIZE, (50 * 1024 * 1024, 50 * 1024 * 1024))
        # 4. 限制最大子进程数 (100个，绝对防御 Fork 炸弹)
        resource.setrlimit(resource.RLIMIT_NPROC, (100, 100))

    def clone_repository(self):
        """将目标项目拉取到无菌病房"""
        if os.path.exists(self.workspace):
            shutil.rmtree(self.workspace)
        
        print(f"\n[隔离舱] 正在拉取目标源码: {self.repo_url}")
        try:
            # 只拉取最近 1 次 commit，极速 Clone，节省磁盘
            # 注意：Git Clone 不受沙盒限制，由宿主机直接执行以保证网络通畅
            subprocess.run(["git", "clone", "--depth", "1", self.repo_url, self.workspace],
                check=True, capture_output=True, text=True, timeout=120
            )
            print(f"[隔离舱] 源码已安全锚定至: {self.workspace}")
            return True
        except subprocess.CalledProcessError as e:
            print(f"[隔离舱 Error] 源码拉取失败: {e.stderr.strip()}")
            return False
        except Exception as e:
            print(f"[隔离舱 Error] 源码拉取异常: {e}")
            return False

    def run_safe_command(self, cmd_list):
        """在防爆环境下执行任意指令 (如 pytest, python script.py)"""
        print(f"[隔离舱] 执行指令: {' '.join(cmd_list)}")
        try:
            process = subprocess.Popen(
                cmd_list,
                cwd=self.workspace,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                preexec_fn=self._set_strict_limits, # 注入防爆限制
                text=True
            )
            
            stdout, stderr = process.communicate(timeout=65)
            
            if process.returncode == 0:
                return True, stdout.strip()
            elif process.returncode == -9: # 被系统 SIGKILL
                return False, "FATAL: 进程被内核强制击杀 (可能触发了内存或 CPU 红线)。"
            else:
                return False, stderr.strip() if stderr else stdout.strip()
                
        except subprocess.TimeoutExpired:
            process.kill()
            return False, "TIMEOUT: 物理执行超时 (超过 60s 强制熔断)。"
        except Exception as e:
            return False, f"SYSTEM_ERROR: {str(e)}"

    def cleanup(self):
        """焚毁病房，清除污染"""
        if os.path.exists(self.workspace):
            shutil.rmtree(self.workspace)
            print(f"[隔离舱] 物理清理完毕: {self.workspace}")
