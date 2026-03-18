import redis
import requests
import os
from core.config import settings

def check_redis():
    print(f"[*] 正在探测 Redis (.02) -> {settings.REDIS_HOST}...")
    try:
        r = redis.Redis(host=settings.REDIS_HOST, port=settings.REDIS_PORT, password=settings.REDIS_PASS, socket_timeout=5)
        if r.ping():
            print("[OK] Redis 链路通畅！")
    except Exception as e:
        print(f"[ERR] Redis 连接失败: {e}")

def check_tuck():
    print(f"[*] 正在探测 Tuck 塔台 (.54) -> {settings.TUCK_HOST}...")
    headers = {"Authorization": f"Bearer {settings.TUCK_API_KEY}"}
    url = f"{settings.TUCK_HOST.rstrip('/')}/v1/models"
    try:
        res = requests.get(url, headers=headers, timeout=10)
        if res.status_code == 200:
            print("[OK] Tuck 塔台鉴权通过！")
            print(f"[I] 可用模型: {[m['id'] for m in res.json().get('data', [])]}")
        else:
            print(f"[ERR] Tuck 拒绝访问 (错误码: {res.status_code})")
            print(f"      详情: {res.text}")
    except Exception as e:
        print(f"[ERR] Tuck 通信异常: {e}")

if __name__ == "__main__":
    check_redis()
    print("-" * 30)
    check_tuck()
