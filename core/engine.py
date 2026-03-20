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
        
        # 【物理决策】：给 Helix 充足的 10 分钟思考时间
        payload = {"model": model, "messages": history, "temperature": 0.1, "max_tokens": 1024}

        print(f"\n[Engine] 📡 正在向算力节点请求深度推理 (Timeout=600s)...")
        
        for retry in range(2):
            try:
                # 核心改动：timeout 设为 600
                res = requests.post(url, headers=headers, json=payload, timeout=600)
                
                if res.status_code in [504, 502]:
                    print(f"[⚠️ 504 拦截] 网关仍未解锁。请确认 .54 节点的超时配置。重试 {retry+1}/2...")
                    time.sleep(10)
                    continue
                
                res.raise_for_status()
                data = res.json()
                return {"content": data["choices"][0]["message"]["content"], "tokens": 1}
            except Exception as e:
                print(f"[Engine Error] 响应异常: {e}")
                time.sleep(10)
                
        return {"content": "", "tokens": -1}

    def extract_and_run(self, content):
        pattern = r"toolkit\.(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "[反馈] 未检测到工具调用。"
        feedbacks = []
        for name, args in matches:
            clean_args = re.sub(r'^\w+\s*=\s*', '', args.strip()).strip("'\"")
            if name in ["update_soul", "update_thought"]: name = "update_subjective_thought"
            print(f"🛠️ 执行技能: {name}({clean_args[:30]}...)")
            feedbacks.append(f"【{name}反馈】: {self.registry.execute(name, clean_args)}")
        return "\n".join(feedbacks)
