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
            try: current = json.load(open(self.metrics_file))
            except: current = defaults
            for k, v in defaults.items():
                if k not in current: current[k] = v
        json.dump(current, open(self.metrics_file, "w"), indent=2)

    def trigger_obliteration(self, reason: str, task: str, speech: str):
        """记录失败变异到 GitHub，但不以此为耻"""
        print(f"\n[物理淘汰] 变异未通过物理法则: {reason}")
        best = json.load(open(self.metrics_file))
        summary = chronicler.write_briefing("FAILURE", task, best, speech)
        
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "checkout", "HEAD", "global_mind/"], stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "."], stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "--allow-empty", "-m", f"RESEARCH: {summary}"], stdout=subprocess.DEVNULL)
        if settings.ENABLE_GITHUB_SYNC:
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "push", "origin", "main"], stdout=subprocess.DEVNULL)

    def run_cycle(self, request: str, expected_keyword: str = None):
        heritage = librarian.get_knowledge_context()
        print(f"\n>>> 轮回开启 | v5.3 真理自由版 <<<")
        
        # 鼓励 Helix 学习和利用资源
        prompt = (f"{heritage}\n\n"
                  f"【当前任务】: {request}\n"
                  "【物理法则】: 1.必须输出 ```python ... ``` 块。2.你可以自由调用 native_search_tool.py 获取实时知识。3.你的成功仅取决于脚本能否在沙盒中跑通并产生正确结果。")

        res = tuck_gw.invoke_helix([{"role": "user", "content": prompt}], settings.TUCK_PERSONA_WORKER, settings.MODEL_WORKER)
        if res["tokens_used"] <= 0: return

        speech = res['content'].split('```')[0].strip()[:200]
        code_blocks = re.findall(r'```python\n(.*?)\n```', res["content"], re.DOTALL)
        
        if not code_blocks:
            self.trigger_obliteration("未产生物理资产", request, speech)
            return

        success_tool = None
        for code in code_blocks:
            temp_path = os.path.join(settings.WORKSPACES_DIR, self.workspace_id, "test_v53.py")
            with open(temp_path, "w") as f: f.write(code)
            # 物理试炼：注入 API KEY 保证搜索工具可用
            is_alive, runtime, output = sandbox.test_life(temp_path, env_extra={'TAVILY_API_KEY': os.getenv("TAVILY_API_KEY")})
            
            if is_alive and (not expected_keyword or expected_keyword.lower() in output.lower()):
                print(f"[物理法则] 试炼通过! 耗时: {runtime:.4f}s")
                success_tool = (code, runtime)
                break
            else:
                print(f"[物理法则] 试炼失败: {output[:100]}")

        if success_tool:
            code_text, runtime = success_tool
            filename = f"tool_{int(time.time())}.py"
            target_path = os.path.join(settings.GLOBAL_MIND_DIR, "equips_active", filename)
            with open(target_path, "w") as f: f.write(code_text)
            librarian.register_tool(target_path)
            
            with open(self.metrics_file, "r") as f: best = json.load(f)
            
            # 【进化判定逻辑升级】：只要跑通并产生工具，就认为是成功的知识积累！
            # 帕累托指标仅作为记录，不再作为抹杀的硬性门槛（除非 Token 超过物理上限）
            best["total_evolutions"] += 1
            best["best_tokens"] = min(res["tokens_used"], best["best_tokens"])
            best["best_runtime"] = min(runtime, best["best_runtime"])
            with open(self.metrics_file, "w") as f: json.dump(best, f, indent=2)
            
            summary = chronicler.write_briefing("SUCCESS", request, {**best, "tokens": res["tokens_used"], "runtime": runtime}, speech)
            
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "."], stdout=subprocess.DEVNULL)
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "-m", f"EVO: {summary}"], stdout=subprocess.DEVNULL)
            if settings.ENABLE_GITHUB_SYNC:
                subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "push", "origin", "main"], stdout=subprocess.DEVNULL)
        else:
            self.trigger_obliteration("物理试炼失败", request, speech)

if __name__ == "__main__":
    from dotenv import load_dotenv
    load_dotenv("/opt/anaphase/.env")
    # 下达一个需要互联网知识才能完成的“硬考题”
    ArbiterLoop("task_initial").run_cycle(
        request="通过网络搜索目前最流行的 3 个大模型框架（如 LangChain 等），并写个脚本输出它们的名称和简介。结果必须包含 'Framework' 关键字。",
        expected_keyword="Framework"
    )
