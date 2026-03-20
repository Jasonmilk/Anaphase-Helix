import requests, time, re, os
from core.config import settings

class ExecutionEngine:
    def __init__(self, registry):
        self.registry = registry
        self.settings = settings

    def get_decision(self, history, retry_count=0):
        # 逻辑：如果重试次数 > 0，且有 8B 模型可用，自动升档
        use_heavy = retry_count > 0
        
        if use_heavy:
            model = os.getenv("TUCK_MODEL_AUDITOR", "DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf")
            print(f"[Engine] ⚠️ 检测到逻辑阻塞，正在切换至 8B 推理模型...")
        else:
            model = self.settings.TUCK_MODEL
            print(f"[Engine] 🖥️ 使用 4B 快速执行模型...")

        url = self.settings.TUCK_HOST
        if not url.endswith('/completions'): url = f"{url.rstrip('/')}/v1/chat/completions"
        headers = {"Authorization": f"Bearer {self.settings.TUCK_API_KEY}", "Content-Type": "application/json"}
        
        payload = {
            "model": model,
            "messages": history,
            "temperature": 0.1,
            "max_tokens": 1500
        }

        # 针对 504 的接力重试
        for i in range(1, 4):
            try:
                res = requests.post(url, headers=headers, json=payload, timeout=240)
                if res.status_code == 504:
                    print(f"[⚠️ 504] 网关超时 (尝试 {i}/3)...")
                    time.sleep(5)
                    continue
                res.raise_for_status()
                data = res.json()
                return {"content": data["choices"][0]["message"]["content"], "tokens": 1}
            except Exception as e:
                print(f"[Engine Error] {e}")
                time.sleep(5)
        
        return {"content": "", "tokens": -1}

    def extract_and_run(self, content):
        """增强解析：清洗模型产生的 'args:' 这种幻觉"""
        pattern = r"toolkit\.(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "Thinking..."
        
        results = []
        for name, args in matches:
            # 【关键修复】：如果模型输出了 args: "path" 这种废话，强行清洗掉
            clean_args = args.replace("args:", "").replace("args =", "").strip()
            clean_args = clean_args.strip("'\"")
            
            print(f"🛠️ 执行: {name}({clean_args[:30]}...)")
            res = self.registry.execute(name, clean_args)
            results.append(f"【{name}反馈】: {res}")
        return "\n".join(results)
