import subprocess
import time
import os
import sys

class SandboxRunner:
    """物理沙盒：执行脚本并测量生命体征 (运行时间、内存、返回码)"""
    
    def __init__(self, timeout=10):
        self.timeout = timeout

    def test_life(self, script_path):
        """
        测试脚本是否具有生命活力（能跑通）
        返回: (是否存活, 运行耗时, 错误信息)
        """
        start_time = time.time()
        try:
            # 执行脚本并捕获输出
            result = subprocess.run(
                [sys.executable, script_path],
                capture_output=True,
                text=True,
                timeout=self.timeout
            )
            runtime = time.time() - start_time
            
            if result.returncode == 0:
                # 成功运行，视为具有初步生命活力
                return True, runtime, result.stdout.strip()
            else:
                return False, runtime, result.stderr.strip()
                
        except subprocess.TimeoutExpired:
            return False, self.timeout, "PHYSICAL_TIMEOUT: 运行过慢，已被物理法则淘汰。"
        except Exception as e:
            return False, 0, f"ENVIRONMENT_ERROR: {str(e)}"

sandbox = SandboxRunner()
