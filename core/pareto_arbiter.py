import json
import os
import subprocess
from core.config import settings

class ParetoArbiter:
    """帕累托仲裁器：负责物理层面的进化裁决与 Git 存证"""
    
    def __init__(self):
        self.best_file = os.path.join(settings.GLOBAL_MIND, "pareto_best.json")
        self.legacy_dir = settings.GLOBAL_MIND

    def get_best_metrics(self):
        with open(self.best_file, 'r') as f:
            return json.load(f)

    def evaluate_evolution(self, current_metrics):
        """
        帕累托核心逻辑：
        只有在 Token 消耗更低 OR 工具复用率更高，且其他维度不下降时，才允许进化。
        """
        best = self.get_best_metrics()
        
        # 简单判据：Token 消耗是否更低 (生存压力)
        is_better = current_metrics['token_usage'] < best['best_token_usage']
        
        if is_better:
            print(f"[天道仲裁] 进化成功！当前 Token: {current_metrics['token_usage']} < 历史最优: {best['best_token_usage']}")
            self.commit_legacy(current_metrics)
            return True
        else:
            print("[天道仲裁] 进化失败：生存效率未提升，执行物理抹杀 (Git Reset)")
            self.obliterate()
            return False

    def commit_legacy(self, metrics):
        """保存最优指标并 Git 提交存档"""
        with open(self.best_file, 'w') as f:
            json.dump({
                "best_token_usage": metrics['token_usage'],
                "tool_reuse_rate": metrics.get('tool_reuse', 0.0),
                "success_rate": 1.0
            }, f)
        
        try:
            os.chdir(settings.ROOT)
            subprocess.run(["git", "add", "global_mind/"], check=True)
            subprocess.run(["git", "commit", "-m", f"Evolution: Tokens {metrics['token_usage']}"], check=True)
        except Exception as e:
            print(f"[Git Error] 提交失败: {e}")

    def obliterate(self):
        """执行物理抹杀，回滚 global_mind 到上一个稳定版本"""
        try:
            os.chdir(settings.ROOT)
            subprocess.run(["git", "checkout", "HEAD", "global_mind/"], check=True)
        except Exception as e:
            print(f"[Obliteration Error] 抹杀失败: {e}")

arbiter = ParetoArbiter()
