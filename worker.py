import os, time, requests, json, traceback
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline
from core.dashboard import print_header

MIND_URL = "http://127.0.0.1:8020"

def execute_task(task_id, content):
    """前线突击队的战术执行流水线 (8B -> 2B -> 7B)"""
    workspace = os.path.join(settings.WORKSPACES_DIR, f"task_{task_id}")
    os.makedirs(workspace, exist_ok=True)
    
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(task_id)
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)

    print_header(age, cycle, task_id)

    # 严格节流加载认知 (保护 8K 窗口)
    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    thoughts = memory.get_distilled_thoughts()
    skills_docs = registry.get_docs()

    base_history =[
        {"role": "system", "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【思想】:\n{thoughts}"},
        {"role": "user", "content": f"任务目标: {content[:400]}"}
    ]

    valid = False
    error_reason = "任务执行正常。"

    # 工业流水线：脑 -> 眼 -> 手
    for turn in range(1, 6):
        print(f"\n" + "-"*60)
        
        # [8B 大脑规划]
        print(f"🧠[回合 {turn}] 8B-R1 大脑规划中...\n")
        brain_prompt = base_history +[{"role": "user", "content": f"能力库:\n{skills_docs}\n请深思熟虑，给出下一步计划。不写代码。"}]
        raw_plan = engine.get_decision(brain_prompt, role='brain')
        
        if not raw_plan:
            error_reason = f"第 {turn} 回合：8B 大脑节点失去响应 (可能触发 504 物理屏障)。"
            print(f"❌ {error_reason}")
            break
            
        # [2B 眼睛脱水]
        print(f"\n👁️[回合 {turn}] 2B 眼睛脱水中...")
        eyes_prompt =[
            {"role": "system", "content": "你是专业文本提取器，仅做冗余删除。绝对保留原文指令。绝对禁止JSON或代码块。"},
            {"role": "user", "content": f"请提取核心动作指令：\n{raw_plan[:800]}"}
        ]
        dehydrated_plan = engine.get_decision(eyes_prompt, role='eyes')
        if not dehydrated_plan: dehydrated_plan = raw_plan[-200:]
        dehydrated_plan = dehydrated_plan.replace("```json", "").replace("```", "").strip()
        print(f"\n✅ [脱水计划]: {dehydrated_plan.replace(chr(10), ' ')}")
        
        # [7B 双手执行]
        print(f"🦾[回合 {turn}] 7B-Coder 物理执行...\n")
        hands_prompt = base_history +[
            {"role": "system", "content": f"你是双手。规范:\n{skills_docs}\n必须输出 toolkit.xxx()。"},
            {"role": "user", "content": f"执行物理动作:\n{dehydrated_plan}"},
            {"role": "assistant", "content": "toolkit."} 
        ]
        action = engine.get_decision(hands_prompt, role='hands')
        
        if not action:
            error_reason = f"第 {turn} 回合：7B 双手节点未产出有效代码。"
            print(f"❌ {error_reason}")
            break
            
        valid = True
        full_action = "toolkit." + action 
        feedback = engine.extract_and_run(full_action)
        
        # 物理留存：永远不截断的真实档案
        with open(os.path.join(workspace, "full_log.txt"), "a") as f:
            f.write(f"\n--- Turn {turn} ---\n[8B Plan]\n{raw_plan}\n[2B Dehydrated]\n{dehydrated_plan}\n[7B Action]\n{full_action}\n[Feedback]\n{feedback}\n")
        
        print(f"\n📝 物理反馈: {feedback[:150].replace(chr(10), ' ')}...")
        
        base_history.append({"role": "assistant", "content": f"决定：{dehydrated_plan}"})
        base_history.append({"role": "user", "content": f"反馈: {feedback[:300]}"})

    # --- 最终战报生成 (使用 2B 模型) ---
    print("\n📜 正在生成最终战报汇报至认知中枢...")
    if valid:
        summary_prompt =[
            {"role": "system", "content": "你是极简战报撰写员。直接说明最终达成了什么物理结果。限 100 字。"},
            {"role": "user", "content": f"基于以下执行历史，总结任务最终成果：\n{str(base_history[-4:])}"}
        ]
        final_report = engine.get_decision(summary_prompt, role='eyes')
        if not final_report: final_report = "任务已执行，但战报生成失败。"
        print(f"✅ 战报: {final_report}")
        return True, final_report
    else:
        return False, error_reason

def worker_loop():
    print("🦶 洛无忌执行触手 (Anaphase-Tentacle) 已就绪，监听 127.0.0.1:8020 ...")
    while True:
        try:
            resp = requests.get(f"{MIND_URL}/v1/mind/todo/pop", timeout=5)
            if resp.status_code == 200:
                data = resp.json()
                if data.get("has_task"):
                    task_info = data["task"]
                    task_id = task_info["id"]
                    content = task_info["content"]
                    
                    print(f"\n[+] 📡 接收到中央司令部任务 | ID: {task_id}")
                    
                    # 绝对的防爆装甲，保护 Worker 进程永不退出
                    try:
                        is_success, result_detail = execute_task(task_id, content)
                    except Exception as e:
                        is_success = False
                        result_detail = f"Anaphase 内部致命异常: {str(e)}\n{traceback.format_exc()}"
                        print(f"❌ {result_detail}")
                    
                    print(f"\n📡 正在向认知中枢汇报战果...")
                    requests.post(f"{MIND_URL}/v1/mind/report", json={
                        "task_id": task_id,
                        "status": "success" if is_success else "failed",
                        "detail": result_detail[:1500] 
                    }, timeout=5)
                    print(f"[-] 任务 {task_id} 闭环达成。")
                    continue
                    
            time.sleep(5)
            
        except requests.exceptions.RequestException:
            print("⚠️ 认知中枢 (Helix-Mind) 连接异常，正在重试...", end="\r")
            time.sleep(10)

if __name__ == "__main__":
    worker_loop()
