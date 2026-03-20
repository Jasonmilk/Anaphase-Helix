import os
from dotenv import load_dotenv

# 动态定位 .env 路径（核心安全点：避免硬编码）
dotenv_path = os.path.join(os.path.dirname(__file__), '..', '.env')
load_dotenv(dotenv_path)

class Settings:
    # --- 塔台通信 ---
    TUCK_HOST = os.getenv("TUCK_HOST")
    TUCK_API_KEY = os.getenv("TUCK_API_KEY")
    TUCK_URL = f"{TUCK_HOST}/v1/chat/completions" if TUCK_HOST else None  # 补充：直接拼接 API 端点
    
    # --- 模型番号映射 (对齐 .env) ---
    MODEL_WORKER = os.getenv("TUCK_MODEL_WORKER")
    MODEL_AUDITOR = os.getenv("TUCK_MODEL_AUDITOR")
    MODEL_REFINER = os.getenv("TUCK_MODEL_REFINER")
    
    # --- 人格面具 ---
    TUCK_PERSONA_WORKER = os.getenv("TUCK_PERSONA_WORKER", "Persona_Helix_Worker")
    TUCK_PERSONA_AUDITOR = os.getenv("TUCK_PERSONA_AUDITOR", "Persona_Helix_Auditor")

    # --- 外部感官 ---
    TAVILY_API_KEY = os.getenv("TAVILY_API_KEY")
    GITHUB_TOKEN = os.getenv("GITHUB_TOKEN")  # 补充：从 .env 读取 GitHub Token

    # --- 物理路径 ---
    ANAPHASE_ROOT = os.getenv("ANAPHASE_ROOT", "/opt/anaphase")
    GLOBAL_MIND_DIR = os.path.join(ANAPHASE_ROOT, "global_mind")
    WORKSPACES_DIR = os.path.join(ANAPHASE_ROOT, "workspaces")
    
    # --- 进化法则约束 (补充：从 .env 读取) ---
    MAX_TOKENS_PER_LIFESPAN = int(os.getenv("MAX_TOKENS_PER_LIFESPAN", 8000))
    
    # --- 自动化配置 ---
    ENABLE_GITHUB_SYNC = os.getenv("ENABLE_GITHUB_SYNC", "False").lower() == "true"

settings = Settings()
