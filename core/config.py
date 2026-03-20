import os
from dotenv import load_dotenv

# 加载物理环境参数
load_dotenv("/opt/anaphase/.env")

class Settings:
    ANAPHASE_ROOT = os.getenv("ANAPHASE_ROOT", "/opt/anaphase")
    WORKSPACES_DIR = os.path.join(ANAPHASE_ROOT, "workspaces")
    GLOBAL_MIND_DIR = os.path.join(ANAPHASE_ROOT, "global_mind")
    
    # --- 本地矩阵配置 (Tuck) ---
    TUCK_HOST = os.getenv("TUCK_HOST", "http://10.0.0.54:8686")
    TUCK_API_KEY = os.getenv("TUCK_API_KEY", "")
    # 默认使用 4B 量化版作为行动执行官
    TUCK_MODEL = os.getenv("TUCK_MODEL_WORKER", "Qwen3.5-4B-Chat-Q4_0.gguf")
    
    # --- 商业云端配置 (可选) ---
    ENABLE_COMMERCIAL = os.getenv("ENABLE_COMMERCIAL", "False").lower() == "true"
    COMMERCIAL_URL = os.getenv("COMMERCIAL_URL", "https://api.deepseek.com/v1/chat/completions")
    COMMERCIAL_KEY = os.getenv("COMMERCIAL_KEY", "")
    COMMERCIAL_MODEL = os.getenv("COMMERCIAL_MODEL", "deepseek-chat")
    
    # --- 外部授权 ---
    GITHUB_TOKEN = os.getenv("GITHUB_TOKEN", "")
    TAVILY_API_KEY = os.getenv("TAVILY_API_KEY", "")
    ENABLE_GITHUB_SYNC = os.getenv("ENABLE_GITHUB_SYNC", "False").lower() == "true"

settings = Settings()
