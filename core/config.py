import os
from dotenv import load_dotenv

load_dotenv(os.path.join(os.path.dirname(__file__), '..', '.env'))

class Settings:
    TUCK_HOST = os.getenv("TUCK_HOST")
    TUCK_KEY = os.getenv("TUCK_API_KEY")
    
    REDIS_HOST = os.getenv("REDIS_HOST")
    REDIS_PORT = int(os.getenv("REDIS_PORT", 6379))
    REDIS_PASS = os.getenv("REDIS_PASS")
    
    ROOT = "/opt/anaphase"
    GLOBAL_MIND = "/opt/anaphase/global_mind"
    WORKSPACES = "/opt/anaphase/workspaces"
    
    # 【物理生存法则】
    MAX_TOKEN_LIFESPAN = 8000  # 单次轮回 Token 熔断线
    SURVIVAL_REWARD_WEIGHT = 1.2 # 成功调用工具的生存奖励系数

settings = Settings()
