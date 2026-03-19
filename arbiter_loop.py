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
        self._init_metrics()
        cortex.init_lifespan(self.workspace_id)

    def _init_metrics(self):
        defaults = {"best_tokens": 999999, "best_runtime": 999.0, "total_evolutions": 0}
        if not os.path.exists(self.metrics_file): current = defaults
        else:
            with open(self.metrics_file, "r") as f:
                try: current = json.load(f)
                except: current = defaults
        with open(self.metrics_file, "w") as f: json.dump(current, f, indent=2)

    def _extract_code(self, content, workspace_id):
        pending_dir = os.path.join(settings.WORKSPACES_DIR, workspace_id, "equips_pending")
        os.makedirs(pending_dir, exist_ok=True)
        code_blocks = re.findall(r'```python\n(.*?)\n```', content, re.DOTALL)
        paths = []
        for i, code in enumerate(code_blocks):
            path = os.path.join(pending_dir, f"tool_{int(time.time())}_{i}.py")
            with open(path, 'w') as f: f.write(code)
            paths.append(path)
        return paths

    def trigger_obliteration(self, reason: str, content: str, task: str):
        print(f"\n[物理淘汰] 此个体不适应环境: {reason}")
        # 呼叫 Worker 记录最后的挣扎
        prompt = [{"role": "user", "content": f"你因『{reason}』失败。请写下反思录。"}]
        res = tuck_gw.invoke_helix(prompt, settings.TUCK_PERSONA_WORKER, settings.MODEL_WORKER)
        last_words = re.sub(r'<think>.*?</think>', '', res['content'], flags=re.DOTALL).strip()
        
        best = json.load(open(self.metrics_file))
        summary = chronicler.write_briefing("FAILURE", task, best, last_words)
        
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "checkout", "HEAD", "global_mind/"], stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "."], stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "--allow-empty", "-m", f"RESEARCH: {summary}"], stdout=subprocess.DEVNULL)
        if settings.ENABLE_GITHUB_SYNC:
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "push", "origin", "main"], stdout=subprocess.DEVNULL)

    def run_cycle(self, request: str, expected_keyword: str = None):
        heritage = librarian.get_knowledge_context()
        print(f"\n>>> 轮回开启 | 自然选择模式 v5.1 (隐私隔离版) <<<")
        
        prompt = (f"{heritage}\n\n任务: {request}\n"
                  "物理法则: 必须输出 ```python ... ``` 块。脚本必须独立运行。")

        res = tuck_gw.invoke_helix([{"role": "user", "content": prompt}], settings.TUCK_PERSONA_WORKER, settings.MODEL_WORKER)
        if res["tokens_used"] <= 0: return

        code_paths = self._extract_code(res["content"], self.workspace_id)
        if not code_paths:
            self.trigger_obliteration("未产生物理资产", res["content"], request)
            return

        success_tool = None
        for path in code_paths:
            # 关键：在此处注入私密的 TAVILY_API_KEY，不让它留在脚本文件里
            is_alive, runtime, output = sandbox.test_life(path, env_extra={'TAVILY_API_KEY': os.getenv("TAVILY_API_KEY")})
            
            is_valid = True
            if expected_keyword and expected_keyword.lower() not in output.lower():
                is_valid = False

            if is_alive and is_valid:
                print(f"[物理法则] 试炼通过! 耗时: {runtime:.4f}s")
                success_tool = path
                break
            else:
                print(f"[物理法则] 试炼失败: {output[:50]}")

        if success_tool:
            target_path = os.path.join(settings.GLOBAL_MIND_DIR, "equips_active", os.path.basename(success_tool))
            shutil.move(success_tool, target_path)
            librarian.register_tool(target_path)
            
            with open(self.metrics_file, "r") as f: best = json.load(f)
            best["total_evolutions"] += 1
            best["best_tokens"] = min(res["tokens_used"], best["best_tokens"])
            best["best_runtime"] = min(runtime, best["best_runtime"])
            with open(self.metrics_file, "w") as f: json.dump(best, f, indent=2)
            
            summary = chronicler.write_briefing("SUCCESS", request, {**best, "tokens": res["tokens_used"], "runtime": runtime})
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "."], stdout=subprocess.DEVNULL)
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "-m", f"EVO: {summary}"], stdout=subprocess.DEVNULL)
            if settings.ENABLE_GITHUB_SYNC:
                subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "push", "origin", "main"], stdout=subprocess.DEVNULL)
        else:
            self.trigger_obliteration("物理测试未通过", res["content"], request)

if __name__ == "__main__":
    import os
    from dotenv import load_dotenv
    load_dotenv("/opt/anaphase/.env")
    ArbiterLoop("task_initial").run_cycle(
        request="通过网络搜索 2024 年 Qwen2.5 模型的主要技术改进，并输出总结。结果必须包含 'Qwen' 关键字。",
        expected_keyword="Qwen"
    )
