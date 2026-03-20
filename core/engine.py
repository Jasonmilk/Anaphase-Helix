import requests, time, re, os

class ExecutionEngine:
    def __init__(self, registry):
        from core.config import settings
        self.registry = registry
        self.settings = settings

    def get_decision(self, history, use_commercial=False):
        if use_commercial or self.settings.ENABLE_COMMERCIAL:
            url, key, model = self.settings.COMMERCIAL_URL, self.settings.COMMERCIAL_KEY, self.settings.COMMERCIAL_MODEL
        else:
            url, key, model = self.settings.TUCK_HOST, self.settings.TUCK_API_KEY, self.settings.TUCK_MODEL

        if not url.endswith('/completions'): url = f"{url.rstrip('/')}/v1/chat/completions"
        headers = {"Authorization": f"Bearer {key}", "Content-Type": "application/json"}
        payload = {"model": model, "messages": history, "temperature": 0.2, "max_tokens": 1500}

        # 【核心策略】：利用 3 次重试，每次重试都会让 Node .02 缓存更多 Token
        # 即使 .54 在 60s 掐断，第 2/3 次重试会直接从缓存开始，从而击穿 504 屏障
        for retry in range(1, 4):
            try:
                print(f"\n[Engine] 尝试第 {retry}/3 次击穿物理屏障...")
                res = requests.post(url, headers=headers, json=payload, timeout=300)
                
                if res.status_code == 504:
                    print(f"[⚠️ 504] 网关超时。但 .02 节点的 KV Cache 已沉淀。5秒后发起接力...")
                    time.sleep(5)
                    continue
                
                res.raise_for_status()
                data = res.json()
                return {"content": data["choices"][0]["message"]["content"], "tokens": 1}
            except Exception as e:
                if "504" in str(e):
                    time.sleep(5)
                    continue
                print(f"\n[Engine Error] 物理链路异常: {e}")
                time.sleep(10)
        return {"content": "", "tokens": -1}

    def extract_and_run(self, content):
        pattern = r"(?:toolkit\.)?(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "Helix 正在纯思维演化。"
        
        feedbacks = []
        for name, args in matches:
            if name == 'name': continue
            clean_args = re.sub(r'^\w+\s*=\s*', '', args.strip()).strip("'\"")
            if name in ["update_soul", "update_thought"]: name = "update_subjective_thought"
            res = self.registry.execute(name, clean_args)
            feedbacks.append(f"【{name}反馈】: {res}")
        return "\n".join(feedbacks)
