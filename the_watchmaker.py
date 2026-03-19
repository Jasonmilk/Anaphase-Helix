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
    if not ward.clone_repository(): return
    
    toolkit = SurgeonToolkit(ward.workspace)
    memory = MemoryCortex(ward.workspace)
    controller = MissionController(max_relays=5)
    engine = ExecutionEngine(toolkit)
    
    last_map = ""
    for relay in range(1, controller.max_relays + 1):
        print(f"\n" + "█"*40 + f" [班次 {relay}] " + "█"*40)
        
        cognitive_map = memory.wake_up()
        
        # 【记忆停滞熔断】
        if cognitive_map == last_map and relay > 1:
            print("⚠️ 检测到『认知循环』，强制清除记忆重新开始！")
            cognitive_map = "前世陷入死循环，请换个思路，先搜索本项目最核心的算子定义文件。"
        last_map = cognitive_map

        history = [{"role": "user", "content": "物理环境已重置，请开始操作。"}]
        
        for turn in range(1, 9):
            res = engine.get_decision(cognitive_map, target['title'], target['body'], history)
            
            if res['tokens_used'] <= 0:
                print("504 Timeout - 触发断点保存...")
                break 
            
            history.append({"role": "assistant", "content": res['content']})
            feedback = engine.extract_and_run(res['content'])
            print(f"[Turn {turn}] {feedback[:60]}...")
            history.append({"role": "user", "content": f"【物理反馈】: {feedback}"})
            
            if "凭证路径" in feedback:
                chronicler.write_briefing("SUCCESS", target['title'], {"tokens": res['tokens_used'], "path": feedback})
                return

        # 交接前的脱水
        memory.dehydrate(history, "轮次耗尽。")
        controller.hibernate(f"班次 {relay} 挂起。")

if __name__ == "__main__":
    run_relay_mission()
