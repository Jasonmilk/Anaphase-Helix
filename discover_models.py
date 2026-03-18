import requests
import json
from core.config import settings

url = f"{settings.TUCK_HOST.rstrip('/')}/v1/models"
headers = {"Authorization": f"Bearer {settings.TUCK_API_KEY}"}

print(f"[*] 正在请求塔台模型清单: {url}...")
try:
    res = requests.get(url, headers=headers, timeout=10)
    if res.status_code == 200:
        models = res.json().get('data', [])
        print("\n[OK] 塔台已响应！当前可用的模型 ID 如下：")
        print("-" * 40)
        for m in models:
            print(f"  >  {m['id']}")
        print("-" * 40)
        print("[!] 请从中挑选出 2B、4B 和 8B 对应的 ID，填入接下来的 .env 中。")
    else:
        print(f"[ERR] 无法获取清单 (错误码: {res.status_code})")
        print(f"详情: {res.text}")
except Exception as e:
    print(f"[ERR] 通信异常: {e}")
