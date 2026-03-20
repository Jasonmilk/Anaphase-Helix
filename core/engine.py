import requests, time, re, os

class ExecutionEngine:
    def __init__(self, registry):
        from core.config import settings
        self.registry = registry
        self.settings = settings

    def get_decision(self, history, use_commercial=False):
        # 1. 引擎寻址 (支持未来商用 API)
        if use_commercial or self.settings.ENABLE_COMMERCIAL:
            url, key, model = self.settings.COMMERCIAL_URL, self.settings.COMMERCIAL_KEY, self.settings.COMMERCIAL_MODEL
        else:
            url, key, model = self.settings.TUCK_HOST, self.settings.TUCK_API_KEY, self.settings.TUCK_MODEL

        if not url.endswith('/completions'): url = f"{url.rstrip('/')}/v1/chat/completions"
        headers = {"Authorization": f"Bearer {key}", "Content-Type": "application/json"}
        
        # 2. 认知代谢：如果对话历史太长，压缩早期的 Feedback
        managed_history = []
        for i, msg in enumerate(history):
            if msg['role'] == 'user' and len(msg['content']) > 1000 and i < len(history) - 2:
                # 对非最近的巨大反馈进行“脱水”处理
                managed_history.append({"role": "user", "content": f"[旧反馈已脱水]: {msg['content'][:100]}... (完整数据见物理海马体)"})
            else:
                managed_history.append(msg)

        payload = {"model": model, "messages": managed_history, "temperature": 0.1, "max_tokens": 1024}

        # 3. 接力执行
        for retry in range(1, 7):
            try:
                res = requests.post(url, headers=headers, json=payload, timeout=300)
                if res.status_code in [502, 504]:
                    print(f"\n[⚠️ 接力 {retry}/6] 缓存预热中...")
                    time.sleep(3)
                    continue
                res.raise_for_status()
                data = res.json()
                return {"content": data["choices"][0]["message"]["content"], "tokens": 1}
            except Exception as e:
                if "504" in str(e): continue
                print(f"\n[Engine Error] {e}"); time.sleep(5)
        return {"content": "", "tokens": -1}

    def extract_and_run(self, content):
        pattern = r"(?:toolkit\.)?(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "Thinking..."
        
        results = []
        for name, args in matches:
            if name == 'name': continue
            clean_args = re.sub(r'^\w+\s*=\s*', '', args.strip()).strip("'\"")
            res = self.registry.execute(name, clean_args)
            # 物理反馈保留，但控制返回给 Prompt 的长度
            results.append(f"【{name}反馈】: {res}")
        return "\n".join(results)
