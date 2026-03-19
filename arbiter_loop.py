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

    def trigger_obliteration(self, reason: str, assistant_content: str, task: str):
        """淘汰处理"""
        print(f"\n[物理淘汰] {reason}")
        # 获取 Helix 的临终反思
        prompt = [{"role": "user", "content": f"你因『{reason}』失败。请用一句话总结你的不甘。"}]
        res = tuck_gw.invoke_helix(prompt, settings.TUCK_PERSONA_WORKER, settings.MODEL_WORKER)
        reflection = re.sub(r'<think>.*?</think>', '', res['content'], flags=re.DOTALL).strip()
        
        best = json.load(open(self.metrics_file))
        summary = chronicler.write_briefing("FAILURE", task, best, reflection)
        
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "checkout", "HEAD", "global_mind/"], stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "."], stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "--allow-empty", "-m", f"RESEARCH: {summary}"], stdout=subprocess.DEVNULL)
        if settings.ENABLE_GITHUB_SYNC:
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "push", "origin", "main"], stdout=subprocess.DEVNULL)

    def run_cycle(self, request: str, expected_keyword: str = None):
        heritage = librarian.get_knowledge_context()
        print(f"\n>>> 轮回开启 | v5.2 史官增强版 <<<")
        
        prompt = (f"{heritage}\n\n任务: {request}\n"
                  "物理法则: 必须输出 ```python ... ``` 块。脚本必须独立运行。")

        res = tuck_gw.invoke_helix([{"role": "user", "content": prompt}], settings.TUCK_PERSONA_WORKER, settings.MODEL_WORKER)
        if res["tokens_used"] <= 0: return

        # 提取感言：取 Helix 回复中代码块前的文字
        speech = res['content'].split('```')[0].strip()
        speech = speech[:200] # 截取前200字作为潜在简报内容

        code_blocks = re.findall(r'```python\n(.*?)\n```', res["content"], re.DOTALL)
        if not code_blocks:
            self.trigger_obliteration("未产生物理资产", res["content"], request)
            return

        success_tool = None
        for code in code_blocks:
            temp_path = os.path.join(settings.WORKSPACES_DIR, self.workspace_id, "test_v52.py")
            with open(temp_path, "w") as f: f.write(code)
            is_alive, runtime, output = sandbox.test_life(temp_path, env_extra={'TAVILY_API_KEY': os.getenv("TAVILY_API_KEY")})
            
            if is_alive and (not expected_keyword or expected_keyword.lower() in output.lower()):
                print(f"[物理法则] 试炼通过! 耗时: {runtime:.4f}s")
                success_tool = (code, runtime)
                break
            else:
                print(f"[物理法则] 试炼失败: {output[:50]}")

        if success_tool:
            code_text, runtime = success_tool
            filename = f"tool_{int(time.time())}.py"
            target_path = os.path.join(settings.GLOBAL_MIND_DIR, "equips_active", filename)
            with open(target_path, "w") as f: f.write(code_text)
            librarian.register_tool(target_path)
            
            with open(self.metrics_file, "r") as f: best = json.load(f)
            best["total_evolutions"] += 1
            best["best_tokens"] = min(res["tokens_used"], best["best_tokens"])
            best["best_runtime"] = min(runtime, best["best_runtime"])
            with open(self.metrics_file, "w") as f: json.dump(best, f, indent=2)
            
            # 【关键修复】成功时将 Helix 的回复摘要传给史官
            summary = chronicler.write_briefing("SUCCESS", request, {**best, "tokens": res["tokens_used"], "runtime": runtime}, speech)
            
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "add", "."], stdout=subprocess.DEVNULL)
            subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "commit", "-m", f"EVO: {summary}"], stdout=subprocess.DEVNULL)
            if settings.ENABLE_GITHUB_SYNC:
                subprocess.run(["git", "-C", settings.ANAPHASE_ROOT, "push", "origin", "main"], stdout=subprocess.DEVNULL)
        else:
            self.trigger_obliteration("物理测试未通过", res["content"], request)

if __name__ == "__main__":
    from dotenv import load_dotenv
    load_dotenv("/opt/anaphase/.env")
    ArbiterLoop("task_initial").run_cycle(
        request="写个 Python 脚本输出当前系统的网卡 MAC 地址 (脱敏处理，仅输出首位字母)。",
        expected_keyword="MAC"
    )
