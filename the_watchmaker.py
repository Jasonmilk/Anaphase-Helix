import json, os, subprocess
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline
from core.dashboard import print_header

def run_relay():
    with open(os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json"), "r") as f:
        target = json.load(f)[0]
    workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{target['issue_number']}")
    os.makedirs(workspace, exist_ok=True)
    
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(target['issue_number'])
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)

    print_header(age, cycle, target['issue_number'])

    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    thoughts = memory.get_distilled_thoughts()
    skills_docs = registry.get_docs()

    # 基础线索 (控制在极小体积)
    base_history =[{
        "role": "system",
        "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【思想】:\n{thoughts}"
    }, {
        "role": "user",
        "content": f"任务: {target['title']}\n{target['body'][:300]}"
    }]

    valid = False
    for turn in range(1, 6):
        print(f"\n" + "-"*60)
        
        # --- 步骤 1：🧠 8B 大脑制定计划 ---
        print(f"🧠[回合 {turn}] 8B-R1 正在进行全量规划(流式接收中)", end="")
        brain_prompt = base_history +[{"role": "user", "content": f"可用技能:\n{skills_docs}\n请深思熟虑，给出下一步的详细计划。不要写代码。"}]
        raw_plan = engine.get_decision(brain_prompt, role='brain')
        if not raw_plan: break
        
        # --- 步骤 2：👁️ 2B 眼睛为计划脱水 ---
        print(f"👁️ [回合 {turn}] 2B 正在为大脑计划脱水", end="")
        eyes_prompt =[
            {"role": "system", "content": "你是指令提炼员。极其客观简练。"},
            {"role": "user", "content": f"大脑输出了冗长的计划：\n{raw_plan}\n请将其提炼为几个核心动作步骤(限150字)，直接说要做什么，禁止寒暄。"}
        ]
        dehydrated_plan = engine.get_decision(eyes_prompt, role='eyes')
        if not dehydrated_plan: dehydrated_plan = raw_plan[:150]
        print(f"\n✅ [脱水计划]: {dehydrated_plan.replace(chr(10), ' ')}")
        
        # --- 步骤 3：🦾 7B 双手执行代码 ---
        print(f"🦾 [回合 {turn}] 7B-Coder 正在根据脱水计划编写指令", end="")
        hands_prompt = base_history +[
            {"role": "system", "content": f"你是执行双手。工具字典:\n{skills_docs}"},
            {"role": "user", "content": f"请严格根据以下动作步骤输出 toolkit.xxx('...') 代码:\n{dehydrated_plan}"}
        ]
        action = engine.get_decision(hands_prompt, role='hands')
        if not action: break
        
        valid = True
        feedback = engine.extract_and_run(action)
        
        # 物理留存：记录三者的完整协作证据链
        with open(os.path.join(workspace, "full_log.txt"), "a") as f:
            f.write(f"\n--- Turn {turn} ---\n[8B Plan]\n{raw_plan}\n[2B Dehydrated]\n{dehydrated_plan}\n[7B Action]\n{action}\n[Feedback]\n{feedback}\n")
        
        print(f"\n📝 物理反馈: {feedback[:100].replace(chr(10), ' ')}...")
        
        # 注入基础历史 (只保留脱水后的计划和精简的反馈)
        base_history.append({"role": "assistant", "content": f"我决定：{dehydrated_plan}。并已执行。"})
        base_history.append({"role": "user", "content": f"反馈: {feedback[:200]}"})

    if valid:
        print("\n🌙 正在执行文明数据归档与 GitHub 同步...")
        subprocess.run(["python3", "generate_status.py"], cwd="/opt/anaphase")
        try:
            subprocess.run(["git", "add", "."], cwd="/opt/anaphase")
            subprocess.run(["git", "commit", "-m", f"feat: 洛无忌 第{age}世 (8B规划->2B脱水->7B执行)"], cwd="/opt/anaphase")
            subprocess.run(["git", "push", "origin", "main"], cwd="/opt/anaphase")
            print("🚀 GitHub 同步成功。")
        except: pass
    
    print(f"🏁 第 {age} 世使命达成。\n")

if __name__ == "__main__":
    run_relay()
