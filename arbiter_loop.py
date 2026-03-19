import os, json, subprocess, re, time, shutil
from core.config import settings
from core.tuck_gateway import tuck_gw
from core.redis_cortex import cortex
from core.security_gate import SecurityGate
from core.librarian import librarian

class ArbiterLoop:
    def __init__(self, workspace_id: str):
        self.workspace_id = workspace_id
        self.metrics_file = os.path.join(settings.GLOBAL_MIND_DIR, "metrics.json")
        self.gate = SecurityGate(workspace_id)
        self._init_metrics()
        cortex.init_lifespan(self.workspace_id)

    def _init_metrics(self):
        if not os.path.exists(self.metrics_file):
            with open(self.metrics_file, "w") as f: 
                json.dump({"best_tokens": 999999, "knowledge_reuse_count": 0}, f)

    def archive_research_log(self, reason, content, last_words):
        """文明归档：将失败的尝试存入科研日志，不抹杀，只记录"""
        log_dir = os.path.join(settings.GLOBAL_MIND_DIR, "research_logs")
        log_file = os.path.join(log_dir, f"fail_{int(time.time())}.json")
        
        log_data = {
            "time": time.strftime("%Y-%m-%d %H:%M:%S"),
            "fail_reason": reason,
            "failed_attempt": content,
            "helix_reflection": last_words
        }
        with open(log_file, "w", encoding="utf-8") as f:
            json.dump(log_data, f, ensure_ascii=False, indent=2)
        
        # 记录到 Git，让失败也成为文明的一部分
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "global_mind/research_logs/"], stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "-m", f"ResearchLog: Archive failure due to {reason}"], stdout=subprocess.DEVNULL)

    def run_cycle(self, request: str):
        # 1. 知识注入 (Lamarckian Inheritance)
        heritage = librarian.get_knowledge_context()
        print(f"\n>>> 轮回开启 | 文明图书馆模式 | 获取历史遗产中... <<<")
        
        evolutionary_prompt = (
            f"{heritage}\n\n"
            f"【当前任务】: {request}\n"
            "【生存法则】: 你的前辈为你留下了上面的工具。如果能用旧工具解决，请直接调用；"
            "如果不能，请制造新工具并写好函数注释。你的造物将被载入史册。"
        )

        # 2. Worker 执行任务
        res = tuck_gw.invoke_helix([{"role": "user", "content": evolutionary_prompt}], settings.TUCK_PERSONA_WORKER, settings.MODEL_WORKER)
        if res["tokens_used"] <= 0: return

        # 3. 审计与提取
        audit = self.gate.process_pending_equips()
        
        if audit["all_safe"] and audit["approved_count"] > 0:
            # 进化成功：注册新知识
            print(f"[天道仲裁] 发现新发明！正在由馆长录入文明索引...")
            for tool_file in os.listdir(os.path.join(settings.GLOBAL_MIND_DIR, "equips_active")):
                librarian.register_tool(os.path.join(settings.GLOBAL_MIND_DIR, "equips_active", tool_file))
            
            # 存档 Git
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "global_mind/"], stdout=subprocess.DEVNULL)
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "-m", f"Civilization Alpha: New Knowledge Indexed (Tokens: {res['tokens_used']})"], stdout=subprocess.DEVNULL)
            print("[Anaphase] 文明已向前迈进一小步。")
        else:
            # 记录失败反思，但不执行残酷的回滚！我们保留这次尝试，让后辈看到。
            print(f"[Anaphase] 任务未能产出合规资产。执行『失败归档协议』。")
            self.archive_research_log("No Asset / Audit Failed", res["content"], "我需要学习如何更严谨地封装函数。")

if __name__ == "__main__":
    os.makedirs("/opt/anaphase/workspaces/task_initial/equips_pending", exist_ok=True)
    ArbiterLoop("task_initial").run_cycle("统计 /etc 下所有 .conf 文件数量。记得检查库里有没有现成的工具。")
