import requests, time, re, os
from core.config import settings

class ExecutionEngine:
    def __init__(self, registry):
        self.registry = registry
        self.settings = settings

    def get_decision(self, history, use_commercial=False):
        if use_commercial or self.settings.ENABLE_COMMERCIAL:
            url, key, model = self.settings.COMMERCIAL_URL, self.settings.COMMERCIAL_KEY, self.settings.COMMERCIAL_MODEL
        else:
            url, key, model = self.settings.TUCK_HOST, self.settings.TUCK_API_KEY, self.settings.TUCK_MODEL

        if not url.endswith('/completions'): url = f"{url.rstrip('/')}/v1/chat/completions"
        headers = {"Authorization": f"Bearer {key}", "Content-Type": "application/json"}
        
        payload = {"model": model, "messages": history, "temperature": 0.1, "max_tokens": 1500}

        # 【核心改进】：物理隧道扩容至 300 秒，允许 ARM 节点完成长预填充
        for retry in range(2):
            try:
                res = requests.post(url, headers=headers, json=payload, timeout=300)
                res.raise_for_status()
                data = res.json()
                return {"content": data["choices"][0]["message"]["content"], "tokens": 1}
            except requests.exceptions.ReadTimeout:
                print(f"[Engine Warning] 物理节点正在进行长文本预填充，已耗时 300s，尝试重试...")
                continue
            except Exception as e:
                print(f"[Engine Error] {e}")
                time.sleep(5)
        return {"content": "", "tokens": -1}

    def extract_and_run(self, content):
        pattern = r"toolkit\.(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "No tools called."
        
        results = []
        for name, args in matches:
            clean_args = re.sub(r'^\w+\s*=\s*', '', args.strip()).strip("'\"")
            if name in ["update_soul", "update_thought"]: name = "update_subjective_thought"
            
            # 记录原始未截断反馈
            res = self.registry.execute(name, clean_args)
            results.append(f"【{name}反馈】: {res}")
        return "\n".join(results)
