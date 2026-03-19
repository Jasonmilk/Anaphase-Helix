import subprocess
import time
import os
import sys

class SandboxRunner:
    def __init__(self, timeout=30):
        self.timeout = timeout

    def test_life(self, script_path, env_extra=None):
        """执行脚本，注入必要的敏感环境变量"""
        start_time = time.time()
        current_env = os.environ.copy()
        if env_extra:
            current_env.update(env_extra)
            
        try:
            result = subprocess.run(
                [sys.executable, script_path],
                capture_output=True,
                text=True,
                timeout=self.timeout,
                env=current_env
            )
            runtime = time.time() - start_time
            if result.returncode == 0:
                return True, runtime, result.stdout.strip()
            else:
                return False, runtime, result.stderr.strip()
        except subprocess.TimeoutExpired:
            return False, self.timeout, "TIMEOUT: Execution exceeded time limit."
        except Exception as e:
            return False, 0, f"ERROR: {str(e)}"

sandbox = SandboxRunner()
