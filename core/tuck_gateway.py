import requests
import json
from core.config import settings

class TuckGateway:
    """Tuck 动态人格网关：负责与 .54 塔台通信"""
    
    def __init__(self):
        self.api_url = f"{settings.TUCK_HOST}/chat/completions"
        self.headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {settings.TUCK_KEY}"
        }

    def invoke_helix(self, messages, persona="Helix_Worker"):
        """
        :param persona: 对应 Tuck 面板中的 Persona ID (如 Helix_Worker, Helix_Auditor)
        """
        payload = {
            "model": "Qwen3.5-4B-Chat-Q4_0.gguf", # 默认载体
            "messages": messages,
            "temperature": 0.7
        }
        # 如果您在 Tuck 中实现了 X-Tuck-Persona Header 切换逻辑：
        self.headers["X-Tuck-Persona"] = persona 

        try:
            # 这里的 timeout 设置为 300s，确保 Tuck 能处理完
            response = requests.post(self.api_url, headers=self.headers, json=payload, timeout=300)
            response.raise_for_status()
            res_json = response.json()
            
            return {
                "content": res_json['choices'][0]['message']['content'],
                "usage": res_json.get('usage', {}).get('total_tokens', 0)
            }
        except Exception as e:
            print(f"[Tuck Gateway 错误] 通讯失败: {e}")
            return None

tuck_gw = TuckGateway()
