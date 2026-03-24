import os, time, requests, json, traceback
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline
from core.dashboard import print_header, print_progress

MIND_URL = "http://127.0.0.1:8020"

def execute_task(task_id, content):
    workspace = os.path.join(settings.WORKSPACES_DIR, f"task_{task_id}")
    os.makedirs(workspace, exist_ok=True)
    
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(task_id)
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)

    # 1. 预加载分层文档 (吸收审查建议)
    brain_skills = registry.get_docs(mode="brain")
    hands_skills = registry.get_docs(mode="hands")
    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    thoughts = memory.get_distilled_thoughts()

    base_history = [
        {"role": "system", "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【进化思想】:\n{thoughts}"},
        {"role": "user", "content": f"任务目标: {content}"}
    ]

    execution_log = []
    valid = False

    for turn in range(1, 6):
        # --- 🧠 阶段 A: 大脑规划 ---
        print_header(age, cycle, task_id, "Brain Planning")
        brain_prompt = base_history + [{"role": "user", "content": f"可用技能表(精简):\n{brain_skills}\n请规划逻辑步骤。"}]
        raw_plan = engine.get_decision(brain_prompt, role='brain')
        if not raw_plan: break
            
        # --- 🦾 阶段 B: 双手执行 ---
        print_header(age, cycle, task_id, "Hands Working")
        # 物理脱水：仅给 Hands 最近的计划片段
        plan_segment = raw_plan[-800:] if len(raw_plan) > 800 else raw_plan
        hands_prompt = base_history + [
            {"role": "system", "content": f"你是动手执行员。必须且只能输出代码。技能规范:\n{hands_skills}"},
            {"role": "user", "content": f"根据计划执行动作:\n{plan_segment}"},
            {"role": "assistant", "content": "toolkit."} 
        ]
        action = engine.get_decision(hands_prompt, role='hands')
        if not action: break
            
        valid = True
        full_action = "toolkit." + action 
        feedback = engine.extract_and_run(full_action)
        
        # 记录证据链
        execution_log.append(f"Turn{turn}: {full_action} -> {feedback[:50]}")
        with open(os.path.join(workspace, "full_log.txt"), "a") as f:
            f.write(f"\n--- Turn {turn} ---\n[Plan]\n{raw_plan}\n[Action]\n{full_action}\n[Feedback]\n{feedback}\n")
        
        # 3. 记忆压实
        base_history.append({"role": "assistant", "content": f"已执行: {full_action}"})
        base_history.append({"role": "user", "content": f"反馈: {feedback[:200]}"})

    if valid:
        # 最终由 2B 进行战报脱水回传
        summary_prompt = [{"role": "user", "content": f"总结任务最终成果：\n" + "\n".join(execution_log)}]
        final_report = engine.get_decision(summary_prompt, role='eyes')
        return True, final_report if final_report else "任务闭环。"
    return False, "由于物理中断未完成"

def worker_loop():
    lifeline = Lifeline()
    while True:
        try:
            age = lifeline.data.get("total_generations", 0)
            print_header(age, status="洛无忌正在巡逻待机")
            resp = requests.get(f"{MIND_URL}/v1/mind/todo/pop", timeout=5)
            if resp.status_code == 200:
                data = resp.json()
                if data.get("has_task"):
                    task = data["task"]
                    is_success, detail = execute_task(task["id"], task["content"])
                    requests.post(f"{MIND_URL}/v1/mind/report", json={
                        "task_id": task["id"], "status": "success" if is_success else "failed", "detail": detail
                    }, timeout=5)
                    lifeline = Lifeline()
                    continue 
            time.sleep(5)
        except Exception:
            time.sleep(10)

if __name__ == "__main__":
    worker_loop()
