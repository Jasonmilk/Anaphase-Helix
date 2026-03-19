import requests
import json
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
            "temperature": 0.2
        }
        
        try:
            # 【物理常数修正】针对 ARM 算力，将超时极限延长至 600 秒
            print(f"[Tuck] 正在连接塔台，请求模型: {final_model} ... (物理等待中)")
            res = requests.post(self.api_url, headers=headers, json=payload, timeout=600)
            
            if res.status_code == 504:
                print(f"[Tuck 504] 塔台超时：模型思考时间超过了网关限制。")
                return {"content": "TIMEOUT", "tokens_used": -1}
                
            if res.status_code != 200:
                print(f"[Tuck Error] 状态码: {res.status_code} | 详情: {res.text}")
                return {"content": "", "tokens_used": -1}
            
            data = res.json()
            return {
                "content": data["choices"][0]["message"]["content"],
                "tokens_used": data["usage"]["total_tokens"]
            }
        except requests.exceptions.Timeout:
            print(f"[Tuck Exception] 物理超时：600 秒已耗尽，模型未能完成推演。")
            return {"content": "TIMEOUT", "tokens_used": -1}
        except Exception as e:
            print(f"[Tuck Exception] 通信异常: {e}")
            return {"content": "", "tokens_used": -1}

tuck_gw = TuckGateway()
