import os
from dotenv import load_dotenv

dotenv_path = os.path.join(os.path.dirname(__file__), '..', '.env')
load_dotenv(dotenv_path)

class Settings:
    TUCK_HOST = os.getenv("TUCK_HOST")
    TUCK_API_KEY = os.getenv("TUCK_API_KEY")
    
    # 三位一体模型番号
    MODEL_WORKER = os.getenv("TUCK_MODEL_WORKER", "qwen-4b")
    MODEL_AUDITOR = os.getenv("TUCK_MODEL_AUDITOR", "ds-r1")
    MODEL_REFINER = os.getenv("TUCK_MODEL_REFINER", "qwen-2b")
    
    REDIS_HOST = os.getenv("REDIS_HOST")
    REDIS_PORT = int(os.getenv("REDIS_PORT", 6379))
    REDIS_PASS = os.getenv("REDIS_PASS")
    
    TUCK_PERSONA_WORKER = os.getenv("TUCK_PERSONA_WORKER")
    TUCK_PERSONA_AUDITOR = os.getenv("TUCK_PERSONA_AUDITOR")
    
    ANAPHASE_ROOT = os.getenv("ANAPHASE_ROOT")
    GLOBAL_MIND_DIR = os.path.join(ANAPHASE_ROOT, "global_mind")
    WORKSPACES_DIR = os.path.join(ANAPHASE_ROOT, "workspaces")
    MAX_TOKENS_PER_LIFESPAN = int(os.getenv("MAX_TOKENS_PER_LIFESPAN", 8000))
    ENABLE_GITHUB_SYNC = os.getenv("ENABLE_GITHUB_SYNC", "False").lower() == "true"

settings = Settings()
