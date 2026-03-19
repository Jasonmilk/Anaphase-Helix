import json
import requests
from core.config import settings

def debug_request():
    print(f"--- [Anaphase .206 深度自检] ---")
    url = f"{settings.TUCK_HOST.rstrip('/')}/v1/chat/completions"
    headers = {
        "Content-Type": "application/json",
        "X-Tuck-Persona": settings.TUCK_PERSONA_WORKER,
        "Authorization": f"Bearer {settings.TUCK_API_KEY}"
    }
    
    # 模拟 arbiter_loop 的真实 Payload
    payload = {
        "model": settings.MODEL_WORKER,
        "messages": [{"role": "user", "content": "ping"}],
        "temperature": 0.2
    }

    print(f"[*] 最终请求 URL: {url}")
    print(f"[*] 最终请求 Headers: {json.dumps(headers, indent=2)}")
    print(f"[*] 最终请求 Payload: {json.dumps(payload, indent=2)}")
    
    try:
        print("\n[*] 正在发送测试包...")
        res = requests.post(url, headers=headers, json=payload, timeout=30)
        print(f"[+] 塔台返回状态码: {res.status_code}")
        print(f"[+] 塔台原始返回: {res.text}")
    except Exception as e:
        print(f"[-] 请求发生崩溃: {e}")

if __name__ == "__main__":
    debug_request()
