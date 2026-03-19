import json, os, time
from core.config import settings
from core.memory import MemoryCortex
from core.lifecycle import MissionController
from core.engine import ExecutionEngine
from core.surgeon_toolkit import SurgeonToolkit
from core.isolation_ward import IsolationWard
from core.chronicler import chronicler

def run_relay_mission():
    pool_path = os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json")
    with open(pool_path, "r") as f: target = json.load(f)[0]
    
    ward = IsolationWard(str(target['issue_number']), target['repo_url'])
    ward.clone_repository()
    
    toolkit = SurgeonToolkit(ward.workspace)
    memory = MemoryCortex(ward.workspace)
    controller = MissionController(max_relays=5)
    engine = ExecutionEngine(toolkit)
    
    for relay in range(1, controller.max_relays + 1):
        print(f"\n" + "█"*40 + f" [接力班次 {relay}] " + "█"*40)
        
        # 提取压榨后的认知内核
        cognitive_kernel = memory.wake_up()
        print(f"[Memory] 本次班次继承内核：\n{cognitive_kernel}")

        # 班次内部对话：为了省 Token，不继承前世的所有对话，只继承内核
        history = [{"role": "user", "content": "物理环境已苏醒。请基于认知内核继续你的工作。任务目标不变。"}]
        
        for turn in range(1, 11):
            # 动态控制：只有第一次探索才发 Body，接力班次只发 Title 减负
            is_first = (relay == 1 and turn == 1)
            res = engine.get_decision(cognitive_kernel, target['title'], history, is_first)
            
            if res['tokens_used'] <= 0: break 
            
            history.append({"role": "assistant", "content": res['content']})
            feedback = engine.extract_and_run(res['content'])
            print(f"[Turn {turn}] Feedback: {feedback[:60]}...")
            history.append({"role": "user", "content": f"【物理反馈】: {feedback}"})
            
            if "凭证路径" in feedback:
                chronicler.write_briefing("SUCCESS", target['title'], {"tokens": res['tokens_used'], "path": feedback})
                return

        # 班次交接前，执行强制脱水，覆盖 session_notes.json
        memory.dehydrate(history, target['title'])
        controller.hibernate(f"班次 {relay} 结束。")

if __name__ == "__main__":
    run_relay_mission()
