import requests
from core.config import settings

class TuckGateway:
    def __init__(self):
        self.api_url = f"{settings.TUCK_HOST.rstrip('/')}/v1/chat/completions"

    def invoke_helix(self, messages: list, persona: str, model_name: str = None) -> dict:
        """
        100% 串行同步调用。
        发送请求后，Python 进程会阻塞，直到模型返回或彻底超时。
        """
        final_model = model_name if model_name else settings.MODEL_WORKER
        
        headers = {
            "Content-Type": "application/json",
            "X-Tuck-Persona": persona,
            "Authorization": f"Bearer {settings.TUCK_API_KEY}"
        }
        payload = {"model": final_model, "messages": messages, "temperature": 0.2}
        
        try:
            # 针对 ARM 算力，我们将超时设为 600 秒 (10分钟)
            # 这保证了 R1 8B 有充足的时间慢慢磨，而不会中途断连
            print(f"[Tuck] 呼叫 {final_model}... (串行等待中)")
            res = requests.post(self.api_url, headers=headers, json=payload, timeout=600)
            
            if res.status_code != 200:
                print(f"[Tuck Error] {res.status_code}: {res.text}")
                return {"content": "", "tokens_used": -1}
                
            data = res.json()
            return {
                "content": data["choices"][0]["message"]["content"],
                "tokens_used": data["usage"]["total_tokens"]
            }
        except Exception as e:
            print(f"[Tuck Exception] 链路异常: {e}")
            return {"content": "", "tokens_used": -1}

tuck_gw = TuckGateway()
