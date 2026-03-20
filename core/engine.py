import requests, time, re, os

class ExecutionEngine:
    def __init__(self, registry):
        from core.config import settings
        self.registry = registry
        self.settings = settings

    def get_decision(self, history, use_commercial=False):
        """决定使用本地 4B 还是商业云端 API"""
        if use_commercial or self.settings.ENABLE_COMMERCIAL:
            url = self.settings.COMMERCIAL_URL
            key = self.settings.COMMERCIAL_KEY
            model = self.settings.COMMERCIAL_MODEL
            label = "☁️ 商业云端"
        else:
            url = self.settings.TUCK_HOST
            key = self.settings.TUCK_API_KEY
            model = self.settings.TUCK_MODEL
            label = "🖥️ 本地矩阵"

        if not url.endswith('/completions'):
            url = f"{url.rstrip('/')}/v1/chat/completions"

        headers = {"Authorization": f"Bearer {key}", "Content-Type": "application/json"}
        payload = {"model": model, "messages": history, "temperature": 0.1, "max_tokens": 1024}

        print(f"[Engine] {label} 发射 >> 模型: {model}")

        for retry in range(2):
            try:
                res = requests.post(url, headers=headers, json=payload, timeout=120)
                if res.status_code == 401:
                    return {"content": "【物理报错】API Key 鉴权失败，请检查 .env 配置。", "tokens": -1}
                if res.status_code == 504:
                    print(f"[Engine Warning] 504 超时，物理降温重试 {retry+1}/2...")
                    time.sleep(10)
                    continue
                res.raise_for_status()
                data = res.json()
                return {"content": data["choices"][0]["message"]["content"], "tokens": 1}
            except Exception as e:
                print(f"[Engine Error] {e}")
                time.sleep(5)
        
        return {"content": "", "tokens": -1}

    def extract_and_run(self, content):
        """解析工具调用，支持链式执行"""
        pattern = r"toolkit\.(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "[反馈] 未检测到工具调用。"
        
        feedbacks = []
        for name, args in matches:
            # 清理参数中的无用引号和前缀
            clean_args = re.sub(r'^\w+\s*=\s*', '', args.strip()).strip("'\"")
            # 兼容性纠错
            if name in ["update_soul", "update_thought"]: name = "update_subjective_thought"
            
            print(f"🛠️ 执行技能: {name}({clean_args})")
            feedbacks.append(f"【{name}反馈】: {self.registry.execute(name, clean_args)}")
        return "\n".join(feedbacks)
