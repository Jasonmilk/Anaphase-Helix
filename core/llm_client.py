import requests
import json
import time

# 塔台正门地址
TUCK_API_BASE = "http://10.0.0.54:8686/v1/chat/completions"
API_KEY = "sk-GFxLzHMiVa9nswET2yUDNJEvToNroj5pIe49seALkJZBREF2"

def ask_helix(messages, model_name="Qwen3.5-4B-Chat-Q4_0.gguf"):
    """
    全量通过 Tuck 塔台。我们不再逃避超时，而是征服它。
    """
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {API_KEY}"
    }
    
    payload = {
        "model": model_name,
        "messages": messages,
        "temperature": 0.7
    }
    
    start_time = time.time()
    print(f"[Anaphase 突触] 正在通过 Tuck 调用 [{model_name}]...")

    try:
        # 这里的 timeout=600 是 Anaphase 客户端愿意等待 Tuck 处理的时间
        response = requests.post(TUCK_API_BASE, headers=headers, json=payload, timeout=600)
        
        duration = time.time() - start_time
        
        if response.status_code == 200:
            print(f"[Anaphase 突触] 调用成功！耗时: {duration:.2f}s")
            return response.json()['choices'][0]['message']['content']
        else:
            print(f"[Anaphase 塔台反馈异常] HTTP {response.status_code}")
            print(f"详情: {response.text}")
            return None
            
    except requests.exceptions.Timeout:
        print(f"[Anaphase 致命超时] 即使是 600 秒，Tuck 也未能完成任务。请检查 8B 模型是否挂起。")
        return None
    except Exception as e:
        print(f"[Anaphase 突触物理故障] {e}")
        return None
