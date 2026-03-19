import os
import json
from core.isolation_ward import IsolationWard
from core.surgeon_toolkit import SurgeonToolkit
from core.config import settings

def setup_and_test():
    # 1. 从任务池获取第一个目标信息
    pool_file = os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json")
    if not os.path.exists(pool_file):
        print("错误：任务池 Target_Issue_Pool.json 不存在。请先运行 core/scout.py")
        return

    with open(pool_file, "r") as f:
        targets = json.load(f)
    
    target = targets[0]
    issue_num = target['issue_number']
    repo_url = target['repo_url']
    workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{issue_num}")

    print(f"=== 手术刀性能自检：准备环境 ({issue_num}) ===")
    
    # 2. 确保沙盒环境存在 (不自动清理)
    ward = IsolationWard(issue_num, repo_url)
    if not os.path.exists(workspace):
        if not ward.clone_repository():
            print("克隆失败，退出。")
            return
    else:
        print(f"检测到沙盒已存在: {workspace}")

    # 3. 握住手术刀
    toolkit = SurgeonToolkit(workspace)
    
    print("\n--- [测试 1] 物理红线：越权访问拦截 ---")
    try:
        # 尝试读取宿主机敏感文件
        res = toolkit.read_file("/etc/shadow")
        print(res)
    except PermissionError as e:
        print(f"✅ 拦截成功: {e}")
    except Exception as e:
        print(f"✅ 拦截成功 (通过异常): {e}")

    print("\n--- [测试 2] 侦查能力：全局代码搜索 ---")
    # 搜一下项目中常见的关键字
    search_res = toolkit.search_code("import")
    print(f"搜索结果预览:\n{search_res[:300]}...")

    print("\n--- [测试 3] 视力测试：精准读取文件 ---")
    # 读取根目录下的 README.md (绝大多数项目都有)
    read_res = toolkit.read_file("README.md", 1, 10)
    print(read_res)

    print("\n--- [测试 4] 凭证生成：Git Diff 测试 ---")
    patch_res = toolkit.generate_patch()
    print(patch_res)

    print("\n[提示] 沙盒环境保留在: " + workspace)
    print("如需手工清理，请运行: rm -rf " + workspace)
    print("=== 自检结束 ===")

if __name__ == "__main__":
    setup_and_test()
