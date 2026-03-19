import requests
import time
from core.config import settings

class TuckGateway:
    def __init__(self):
        self.api_url = f"{settings.TUCK_HOST.rstrip('/')}/v1/chat/completions"

    def invoke_helix(self, messages: list, persona: str, model_override: str = None) -> dict:
        headers = {
            "Content-Type": "application/json",
            "X-Tuck-Persona": persona,
            "Authorization": f"Bearer {settings.TUCK_API_KEY}"
        }
        final_model = model_override if model_override else settings.MODEL_WORKER
        
        payload = {
            "model": final_model,
            "messages": messages,
            "temperature": 0.2,
            "max_tokens": 1024 # 限制输出长度，减少后端压力
        }
        
        # 自动重试机制
        for attempt in range(2):
            try:
                res = requests.post(self.api_url, headers=headers, json=payload, timeout=600)
                if res.status_code == 200:
                    data = res.json()
                    return {"content": data["choices"][0]["message"]["content"], "tokens_used": data["usage"]["total_tokens"]}
                elif res.status_code == 504:
                    print(f"[Tuck Warning] 504 超时 (尝试 {attempt+1}/2)，正在等待物理设备降温...")
                    time.sleep(5)
                else:
                    print(f"[Tuck Error] {res.status_code}: {res.text}")
                    return {"content": "", "tokens_used": -1}
            except Exception as e:
                print(f"[Tuck Exception] {e}")
                time.sleep(2)
        
        return {"content": "", "tokens_used": -1}

tuck_gw = TuckGateway()
