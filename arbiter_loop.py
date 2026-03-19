import os, json, subprocess, re, time, shutil
from core.config import settings
from core.tuck_gateway import tuck_gw
from core.redis_cortex import cortex
from core.sandbox import sandbox
from core.librarian import librarian
from core.chronicler import chronicler

class ArbiterLoop:
    def __init__(self, workspace_id: str):
        self.workspace_id = workspace_id
        self.metrics_file = os.path.join(settings.GLOBAL_MIND_DIR, "metrics.json")
        self.research_dir = os.path.join(settings.GLOBAL_MIND_DIR, "research_logs")
        os.makedirs(self.research_dir, exist_ok=True)
        self._init_metrics()
        cortex.init_lifespan(self.workspace_id)

    def _init_metrics(self):
        defaults = {"best_tokens": 999999, "best_runtime": 999.0, "total_evolutions": 0}
        if not os.path.exists(self.metrics_file): current = defaults
        else:
            with open(self.metrics_file, "r") as f:
                try: current = json.load(f)
                except: current = defaults
            for k, v in defaults.items():
                if k not in current: current[k] = v
        with open(self.metrics_file, "w") as f: json.dump(current, f, indent=2)

    def trigger_extinction(self, reason: str, code_content: str, output: str, task: str):
        timestamp = int(time.time())
        log_file = os.path.join(self.research_dir, f"extinct_{timestamp}.py")
        header = f'"""\nEXTINCTION RECORD\nReason: {reason}\nOutput: {output}\n"""\n\n'
        with open(log_file, "w") as f: f.write(header + code_content)
        
        with open(self.metrics_file, "r") as f: best = json.load(f)
        # 史官记录并返回摘要
        summary = chronicler.write_briefing("FAILURE", task, best, reason)
        
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "checkout", "HEAD", "global_mind/"], stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "."], stdout=subprocess.DEVNULL)
        # 将史官摘要直接写进 Commit 消息！
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "--allow-empty", "-m", f"RESEARCH_LOG: {summary}"], stdout=subprocess.DEVNULL)
        if settings.ENABLE_GITHUB_SYNC:
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "push", "origin", "main"], stdout=subprocess.DEVNULL)

    def run_cycle(self, request: str, expected_keyword: str = None):
        heritage = librarian.get_knowledge_context()
        print(f"\n>>> 轮回开启 | 史官就位 <<<")
        
        prompt = (f"{heritage}\n\n任务: {request}\n"
                  "物理定律: 必须输出 ```python ... ``` 块。脚本必须独立运行并输出结果。")

        res = tuck_gw.invoke_helix([{"role": "user", "content": prompt}], settings.TUCK_PERSONA_WORKER, settings.MODEL_WORKER)
        if res["tokens_used"] <= 0: return

        code_blocks = re.findall(r'```python\n(.*?)\n```', res["content"], re.DOTALL)
        if not code_blocks:
            self.trigger_extinction("No Code Block", "N/A", "N/A", request)
            return

        success_code = None
        for code in code_blocks:
            temp_path = os.path.join(settings.WORKSPACES_DIR, self.workspace_id, "test.py")
            with open(temp_path, "w") as f: f.write(code)
            is_alive, runtime, output = sandbox.test_life(temp_path)
            
            if is_alive and (not expected_keyword or expected_keyword.lower() in output.lower()):
                print(f"[物理法则] 试炼通过! 耗时: {runtime:.4f}s")
                success_code = (code, runtime)
                break
            else:
                self.trigger_extinction(f"Failed: {output[:50]}", code, output, request)

        if success_code:
            code_content, runtime = success_code
            filename = f"tool_{int(time.time())}.py"
            target_path = os.path.join(settings.GLOBAL_MIND_DIR, "equips_active", filename)
            with open(target_path, "w") as f: f.write(code_content)
            librarian.register_tool(target_path)
            
            with open(self.metrics_file, "r") as f: best = json.load(f)
            best["total_evolutions"] += 1
            best["best_tokens"] = min(res["tokens_used"], best["best_tokens"])
            best["best_runtime"] = min(runtime, best["best_runtime"])
            with open(self.metrics_file, "w") as f: json.dump(best, f, indent=2)
            
            # 史官记录并返回摘要
            summary = chronicler.write_briefing("SUCCESS", request, {**best, "tokens": res["tokens_used"], "runtime": runtime, "total_tools": len(os.listdir(os.path.join(settings.GLOBAL_MIND_DIR, "equips_active")))})

            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "."], stdout=subprocess.DEVNULL)
            # 将成功摘要直接写进 Commit 消息！
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "-m", f"EVO: {summary}"], stdout=subprocess.DEVNULL)
            if settings.ENABLE_GITHUB_SYNC:
                subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "push", "origin", "main"], stdout=subprocess.DEVNULL)

if __name__ == "__main__":
    ArbiterLoop("task_initial").run_cycle(
        request="写一个简单的 Python 脚本，输出当前系统所有网卡的名称 (来自 /proc/net/dev)。",
        expected_keyword="Inter-"
    )
