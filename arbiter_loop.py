import os, json, subprocess, re
from core.config import settings
from core.tuck_gateway import tuck_gw
from core.redis_cortex import cortex
from core.security_gate import SecurityGate

class ArbiterLoop:
    def __init__(self, workspace_id: str):
        self.workspace_id = workspace_id
        self.metrics_file = os.path.join(settings.GLOBAL_MIND_DIR, "metrics.json")
        self.gate = SecurityGate(workspace_id)
        if not os.path.exists(self.metrics_file):
            with open(self.metrics_file, "w") as f: json.dump({"best_tokens": 999999}, f)
        cortex.init_lifespan(self.workspace_id)

    def trigger_obliteration(self, reason: str):
        """抹杀与回滚"""
        print(f"\n[天道裁决] 轮回终止: {reason}")
        # 串行写遗言
        res = tuck_gw.invoke_helix([{"role": "user", "content": "总结教训"}], settings.TUCK_PERSONA_WORKER, settings.MODEL_REFINER)
        if res["tokens_used"] > 0:
            clean_words = re.sub(r'<think>.*?</think>', '', res['content'], flags=re.DOTALL).strip()
            print(f"[Helix 遗言]: {clean_words}")
        # 物理回滚 global_mind
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "checkout", "HEAD", "global_mind/"], stdout=subprocess.DEVNULL)

    def run_cycle(self, request: str):
        print(f"\n>>> 轮回开启 (严格串行模式) <<<")
        
        # 1. Worker (4B) 开始干活
        res = tuck_gw.invoke_helix([{"role": "user", "content": request}], settings.TUCK_PERSONA_WORKER, settings.MODEL_WORKER)
        if res["tokens_used"] <= 0: return

        # 2. 代谢判定 (Redis 记录)
        if cortex.consume_tokens(self.workspace_id, res["tokens_used"]):
            self.trigger_obliteration("Token 耗尽")
            return

        # 3. 军火法庭 (此时 Worker 已退出，Auditor 8B 登场)
        audit = self.gate.process_pending_equips()
        
        # 4. 帕累托终审
        with open(self.metrics_file, "r") as f: best = json.load(f)

        if audit["all_safe"] and (res["tokens_used"] < best["best_tokens"] or audit["approved_count"] > 0):
            print(f"[天道仲裁] 发现进化！Tokens: {res['tokens_used']} | 工具: +{audit['approved_count']}")
            best["best_tokens"] = min(res["tokens_used"], best["best_tokens"])
            with open(self.metrics_file, "w") as f: json.dump(best, f)
            # 仅存档进化结果
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "global_mind/"], stdout=subprocess.DEVNULL)
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "-m", f"Evo: Tokens {res['tokens_used']}"], stdout=subprocess.DEVNULL)
        else:
            self.trigger_obliteration("平庸变异或审计驳回")

if __name__ == "__main__":
    # 创建任务沙盒
    os.makedirs("/opt/anaphase/workspaces/task_initial/equips_pending", exist_ok=True)
    # 开启第一次生命脉动
    ArbiterLoop("task_initial").run_cycle("统计 /etc 下所有 .conf 文件数量，写个函数脚本放入 equips_pending。")
