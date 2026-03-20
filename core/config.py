import os
from dotenv import load_dotenv

load_dotenv("/opt/anaphase/.env")

class Settings:
    ANAPHASE_ROOT = os.getenv("ANAPHASE_ROOT", "/opt/anaphase")
    WORKSPACES_DIR = os.path.join(ANAPHASE_ROOT, "workspaces")
    GLOBAL_MIND_DIR = os.path.join(ANAPHASE_ROOT, "global_mind")
    
    TUCK_API_KEY = os.getenv("TUCK_API_KEY", "脱敏")
    TUCK_HOST = os.getenv("TUCK_HOST", "http://10.0.0.54:8686")
    
    # --- 三位一体模型角色定义 ---
    # 大脑: DeepSeek-R1-8B (审计/规划) - 端口 8016
    MODEL_BRAIN = os.getenv("TUCK_MODEL_AUDITOR", "DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf")
    # 终端: Qwen2.5-Coder-7B (执行/工具) - 端口 8015
    MODEL_HANDS = os.getenv("TUCK_MODEL_WORKER", "Qwen2.5-Coder-7B-Instruct-Q4_0_4_4.gguf")
    # 过滤器: Qwen-2B (脱水/快速处理) - 端口 8014
    MODEL_EYES = os.getenv("TUCK_MODEL_REFINER", "Qwen3.5-2B-IQ4_NL.gguf")

settings = Settings()
