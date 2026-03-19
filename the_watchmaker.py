import json, os, time
from core.config import settings
from core.memory import MemoryCortex
from core.lifecycle import MissionController
from core.engine import ExecutionEngine
from core.surgeon_toolkit import SurgeonToolkit
from core.isolation_ward import IsolationWard
from core.chronicler import chronicler

def run_relay_mission():
    # 1. 加载任务
    pool_path = os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json")
    if not os.path.exists(pool_path):
        print("❌ 错误：未找到任务池。请先运行 scout.py")
        return

    with open(pool_path, "r") as f:
        targets = json.load(f)
    if not targets: return
    target = targets[0]
    
    # 2. 初始化核心组件
    ward = IsolationWard(str(target['issue_number']), target['repo_url'])
    if not ward.clone_repository(): return
    
    toolkit = SurgeonToolkit(ward.workspace)
    memory = MemoryCortex(ward.workspace)
    controller = MissionController(max_relays=5)
    engine = ExecutionEngine(toolkit)
    
    # 3. 开启接力循环
    consecutive_fails = 0
    for relay in range(1, controller.max_relays + 1):
        print(f"\n" + "█"*40 + f" [接力班次 {relay}] " + "█"*40)
        
        # 【强制苏醒协议】
        cognitive_map = memory.wake_up()
        
        # 初始会话历史
        history = [{"role": "user", "content": f"当前物理环境已准备就绪，请开始操作。"}]
        
        # 每个班次限定 8 轮
        for turn in range(1, 9):
            # 对齐 engine.py 的新接口
            res = engine.get_decision(
                cognitive_map=cognitive_map,
                target_title=target['title'],
                target_body=target['body'],
                history=history
            )
            
            if res['tokens_used'] <= 0:
                consecutive_fails += 1
                melted, reason = controller.check_melt(relay, consecutive_fails)
                if melted: 
                    controller.hibernate(reason)
                    return
                break 
            
            consecutive_fails = 0
            history.append({"role": "assistant", "content": res['content']})
            
            # 物理执行
            feedback = engine.extract_and_run(res['content'])
            print(f"[Turn {turn}] Feedback: {feedback[:80]}...")
            history.append({"role": "user", "content": f"【物理反馈】: {feedback}"})
            
            if "凭证路径" in feedback:
                print("🎉 任务达成！")
                chronicler.write_briefing("SUCCESS", target['title'], {"tokens": res['tokens_used'], "path": feedback})
                return

        # 班次交接，执行脱水
        memory.dehydrate(history, "轮次耗尽，准备交接。")
        controller.hibernate(f"班次 {relay} 结束。")

if __name__ == "__main__":
    run_relay_mission()
