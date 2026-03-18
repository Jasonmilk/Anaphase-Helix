import json
import redis
import time
from core.tuck_gateway import tuck_gw
from core.config import settings

def start_consumer():
    r = redis.Redis(host=settings.REDIS_HOST, port=settings.REDIS_PORT, 
                    password=settings.REDIS_PASS, decode_responses=True)
    
    print(f"[*] Anaphase 脑干处理器已启动，正在监听 8B 审计队列...")
    
    while True:
        # 1. 从队列中获取任务
        task_raw = r.brpop("audit_queue", timeout=10)
        if not task_raw:
            continue
            
        task = json.loads(task_raw[1])
        task_id = task['task_id']
        print(f"\n[处理中] 收到 8B 审计任务: {task_id}，正在呼叫 R1 8B 深度思考...")
        
        # 2. 强制执行同步调用 (绕过网关的异步逻辑，直接请求 Tuck)
        # 注意：这里调用的是网关内部的同步方法
        response = tuck_gw._invoke_sync_tuck(
            messages=task['messages'], 
            persona=task['persona'], 
            model=task['model']
        )
        
        # 3. 将结果塞回 Redis 对应的结果键位
        r.lpush(task_id, json.dumps(response))
        # 设置结果 10 分钟过期，防止堆积
        r.expire(task_id, 600)
        print(f"[已完成] 任务 {task_id} 审计结论已返回。")

if __name__ == "__main__":
    start_consumer()
