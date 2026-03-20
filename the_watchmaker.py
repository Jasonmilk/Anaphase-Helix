import json
import os
from core.config import settings
from core.ward import IsolationWard
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline  # 导入生命线

def load_task():
    task_path = os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json")
    if not os.path.exists(task_path): return None
    with open(task_path, "r", encoding="utf-8") as f: tasks = json.load(f)
    return tasks[0] if tasks else None

def run_relay():
    target = load_task()
    if not target: return

    issue_num = target["issue_number"]
    workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{issue_num}")
    os.makedirs(workspace, exist_ok=True)
    
    # 初始化生命线并记录心跳
    lifeline = Lifeline()
    total_age, task_cycle = lifeline.heartbeat(issue_num)
    age_report = f"Helix 第 {total_age} 世 | 任务 #{issue_num} 轮回第 {task_cycle} 轮"

    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)
    
    soul_content = ""
    soul_path = "/opt/anaphase/global_mind/soud.md"
    if os.path.exists(soul_path):
        with open(soul_path, "r", encoding="utf-8") as f: soul_content = f.read()

    skill_docs = registry.get_docs()
    report = memory.wake_up()

    history =[{
        "role": "system",
        "content": f"【当前生命维度】: {age_report}\n【你的灵魂】:\n{soul_content}"
    }, {
        "role": "user",
        "content": f"任务:{target['title']}\n摘要:{target['body'][:150]}...\n指令:开始探索并执行，完成后使用 update_soud 记录进化。"
    }]

    print("\n" + "="*50)
    print(f"🧬 {age_report}")
    print("="*50 + "\n")
    
    valid_interaction = False
    for turn in range(1, 6):
        print(f"[轮回 {turn}/5] 思考中...", end="\r")
        res = engine.get_decision("", target['title'], "", history)
        if not res or res['tokens_used'] <= 0: break
        
        valid_interaction = True
        history.append({"role": "assistant", "content": res["content"]})
        feedback = engine.extract_and_run(res["content"])
        print(f"[T{turn}] 反馈: {feedback[:60]}...")
        history.append({"role": "user", "content": f"【反馈】{feedback[:200]}"})

    if valid_interaction:
        memory.dehydrate(history)
    print(f"\n✅ 任务 #{issue_num} 周期结束。")

if __name__ == "__main__":
    run_relay()
