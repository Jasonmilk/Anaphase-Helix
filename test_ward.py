import json
import os
from core.isolation_ward import IsolationWard
from core.config import settings

def test_isolation():
    pool_file = os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json")
    
    if not os.path.exists(pool_file):
        print("未找到任务池，请先运行 scout.py")
        return

    with open(pool_file, "r") as f:
        targets = json.load(f)

    if not targets:
        print("任务池为空。")
        return

    # 取出雷达锁定的第一个目标
    target = targets[0]
    print(f"--- 隔离舱测试启动 ---")
    print(f"目标 Issue: #{target['issue_number']} - {target['title']}")
    
    # 1. 初始化隔离舱
    ward = IsolationWard(target['issue_number'], target['repo_url'])
    
    # 2. 拉取源码
    if not ward.clone_repository():
        return

    # 3. 在沙盒内执行安全探测 (列出目录文件)
    print("\n[沙盒探测] 尝试在受限环境中执行 'ls -la'...")
    success, output = ward.run_safe_command(["ls", "-la"])
    
    if success:
        print(f"[探测成功] 沙盒内文件结构:\n{output[:300]}...\n(已截断)")
    else:
        print(f"[探测失败] {output}")

    # 4. 销毁病房 (保持环境纯净)
    print("\n[清理阶段]")
    ward.cleanup()
    print("--- 隔离舱测试圆满结束 ---")

if __name__ == "__main__":
    test_isolation()
