import json
import os
from core.config import settings
from core.ward import IsolationWard
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex

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
    
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)
    
    # 【新增：唤醒灵魂】
    soul_content = ""
    soul_path = "/opt/anaphase/global_mind/soud.md"
    if os.path.exists(soul_path):
        with open(soul_path, "r", encoding="utf-8") as f:
            soul_content = f.read()

    skill_docs = registry.get_docs()
    short_body = target['body'][:150].replace('\n', ' ') + "..."
    report = memory.wake_up()
    short_report = report[:100] if len(report) > 100 else report

    # 将灵魂注入到初始 Context 中
    history =[{
        "role": "system",
        "content": f"【你的灵魂与认知】:\n{soul_content}"
    }, {
        "role": "user",
        "content": f"""【当前物理限制】:ARM 1G RAM
【任务摘要】:{target['title']} - {short_body}
【已装备动态技能】:
{skill_docs}
【指令】:请直接输出工具指令(toolkit.xxx)探索或执行，并在完成重大突破后使用 update_soud 记录你的进化思想。严禁废话。"""
    }]

    print("\n=====================================")
    print("🚀 Helix 唤醒 | 灵魂与技能已加载")
    print("=====================================\n")
    
    valid_interaction = False
    for turn in range(1, 6):
        print(f"[轮回 {turn}/5] 正在思考决策中...")
        res = engine.get_decision(short_report, target['title'], target['body'], history)
        if not res or res['tokens_used'] <= 0: break
        
        valid_interaction = True
        print(f"[轮回 {turn}] 执行调用...")
        history.append({"role": "assistant", "content": res["content"]})
        
        feedback = engine.extract_and_run(res["content"])
        print(f"[轮回 {turn}] 反馈：{feedback[:80]}...")
        history.append({"role": "user", "content": f"【反馈】{feedback[:300]}"})

    if valid_interaction:
        memory.dehydrate(history)
    print(f"\n✅ 任务 #{issue_num} 轮回结束！")

if __name__ == "__main__":
    run_relay()
