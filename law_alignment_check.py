import os
from core.config import settings

def verify_intent():
    print("--- Helix 生存法则物理参数检查 ---")
    
    # 1. 检查 GitHub 同步开关
    sync_status = "开启 (自动上传)" if settings.ENABLE_GITHUB_SYNC else "关闭 (仅本地演化)"
    print(f"[!] GitHub 同步: {sync_status}")
    
    # 2. 检查生存压力
    print(f"[!] 生命周期 Token 上限: {settings.MAX_TOKENS_PER_LIFESPAN}")
    
    # 3. 检查三位一体分工
    print(f"[!] 劳工 (Worker): {settings.MODEL_WORKER}")
    print(f"[!] 审计 (Auditor): {settings.MODEL_AUDITOR} (DeepSeek R1)")
    print(f"[!] 提炼 (Refiner): {settings.MODEL_REFINER}")
    
    # 4. 验证 .gitignore 防火墙
    if os.path.exists(".gitignore"):
        with open(".gitignore", "r") as f:
            content = f.read()
            if ".env" in content and "workspaces" in content:
                print("[OK] 隐私防火墙已就绪，安全。")
            else:
                print("[WARN] .gitignore 似乎缺失隐私规则！")
    
    print("\n--- 演化意图对齐 ---")
    print("当前逻辑：无工具不进化。只要 Auditor 报错或 Worker 没写文件，就会触发 Git Reset 回滚。")

if __name__ == "__main__":
    verify_intent()
