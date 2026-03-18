import redis
from core.config import settings

class RedisCortex:
    def __init__(self):
        self.client = redis.Redis(
            host=settings.REDIS_HOST,
            port=settings.REDIS_PORT,
            password=settings.REDIS_PASS,
            decode_responses=True
        )
        
    def init_lifespan(self, task_id: str):
        """初始化一次轮回的生命体征"""
        key = f"helix:lifespan:{task_id}"
        self.client.hset(key, mapping={
            "tokens_used": 0,
            "status": "awake",
            "cycle_count": 1
        })
        # 24小时过期，防止 Redis 膨胀
        self.client.expire(key, 86400)

    def consume_tokens(self, task_id: str, amount: int) -> bool:
        """扣除 Token，返回是否触发疲劳临界点"""
        key = f"helix:lifespan:{task_id}"
        current = self.client.hincrby(key, "tokens_used", amount)
        return current >= settings.MAX_TOKENS_PER_LIFESPAN

cortex = RedisCortex()
