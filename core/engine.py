import requests
import time
import re
import os
from core.config import settings

class ExecutionEngine:
    def __init__(self, registry):
        self.registry = registry # 改为接收技能注册中心
        
        raw_host = getattr(settings, 'TUCK_HOST', getattr(settings, 'API_BASE', os.getenv('TUCK_HOST', 'http://10.0.0.54:8686')))
        self.api_base = raw_host.rstrip('/')
        if not self.api_base.endswith('/v1/chat/completions'):
            self.api_base = f"{self.api_base}/v1/chat/completions"
            
        self.api_key = getattr(settings, 'TUCK_API_KEY', getattr(settings, 'API_KEY', os.getenv('TUCK_API_KEY', 'dummy')))
        self.model = getattr(settings, 'TUCK_MODEL_WORKER', getattr(settings, 'MODEL_WORKER', os.getenv('TUCK_MODEL_WORKER', 'Qwen3.5-4B-Chat-Q4_0.gguf')))
        self.timeout = 120 

    def get_decision(self, memory, title, body, history):
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.api_key}"
        }

        payload = {
            "model": self.model,
            "messages": history,
            "stream": False,
            "temperature": 0.1,
            "max_tokens": 1024 
        }

        for retry in range(1, 3):
            try:
                response = requests.post(self.api_base, headers=headers, json=payload, timeout=self.timeout)
                if response.status_code in [502, 504]:
                    print(f"[Engine Warning] 网关算力过载 (状态码 {response.status_code})。等待降温重试 ({retry}/2)...")
                    time.sleep(5)
                    continue
                
                response.raise_for_status() 
                if response.status_code == 200:
                    data = response.json()
                    return {
                        "tokens_used": data.get("usage", {}).get("total_tokens", 1),
                        "content": data["choices"][0]["message"]["content"]
                    }
            except requests.exceptions.Timeout:
                print(f"[Engine Warning] 物理请求超时 ({retry}/2)...")
                time.sleep(2)
            except Exception as e:
                print(f"[Engine Error] 异常: {e} ({retry}/2)...")
                time.sleep(2)

        print("[Engine] 🚨 物理底座无响应，触发熔断。")
        return {"tokens_used": -1, "content": ""}

    def extract_and_run(self, content):
        pattern = r"toolkit\.(\w+)\((.*?)\)"
        match = re.search(pattern, content)
        if match:
            func_name = match.group(1)
            args_raw = match.group(2)
            # 动态派发给插件中心执行！
            return self.registry.execute(func_name, args_raw)
        
        return f"[测试反馈] 未检测到工具调用 (请使用 toolkit.xxx 格式)。提取内容：{content[:40]}..."
