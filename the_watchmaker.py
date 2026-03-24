import json, os, subprocess, time
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

    base_history =[{
        "role": "system",
        "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【思想】:\n{thoughts}"
    }, {
        "role": "user",
        "content": f"任务目标: {target['title']}\n{target['body'][:300]}"
    }]

    valid = False
    for turn in range(1, 6):
        print(f"\n" + "-"*65)
        
        # --- 🧠 步骤 1：8B 大脑全量规划 ---
        print(f"🧠[回合 {turn}] 8B-R1 正在进行全量规划...\n")
        brain_prompt = base_history +[{"role": "user", "content": f"可用能力:\n{skills_docs}\n请给出严格的逻辑推演与下一步物理指令。用简练的步骤概括，若技能参数(如代码)过长可使用省略号，后续由7B补全。"}]
        raw_plan = engine.get_decision(brain_prompt, role='brain')
        
        if not raw_plan:
            print("❌ 8B 大脑无响应。触发自愈循环...")
            base_history.append({"role": "user", "content": "【系统警告】大脑节点响应失败，请重新思考。"})
            time.sleep(3); continue
            
        # --- ✂️ 步骤 2：物理脱水 (0 Token, 100% 稳定) ---
        # 提取 8B 计划的后 800 字符，过滤掉前面漫长繁杂的推演
        dehydrated_plan = raw_plan[-800:] if len(raw_plan) > 800 else raw_plan
        print(f"\n✅ [物理脱水]: 成功截取大脑核心指令 {len(dehydrated_plan)} 字节。")
        
        # --- 🦾 步骤 3：7B 双手精准执行 ---
        print(f"\n🦾 [回合 {turn}] 7B-Coder 正在翻译为物理动作...\n")
        hands_prompt = base_history +[
            {"role": "system", "content": f"你是物理执行接口。严格遵循:\n{skills_docs}\n大脑指令中可能包含省略号，你必须根据上下文将其补全为完整的 Python 脚本代码！绝对禁止使用 Markdown 代码块，必须直接以 toolkit. 开头！"},
            {"role": "user", "content": f"大脑的最终行动指令:\n{dehydrated_plan}\n\n请输出物理动作:"}
        ]
        
        action = engine.get_decision(hands_prompt, role='hands')
        
        # 【核心保护】：防断裂机制。如果 7B 为空，绝不 break 退出游戏。
        if not action:
            print("\n❌ 7B 终端响应异常 (可能被网关拦截)。正在交由大脑重试...")
            base_history.append({"role": "assistant", "content": f"我进行了规划：\n{dehydrated_plan}"})
            base_history.append({"role": "user", "content": "【系统反馈】7B 物理执行终端未响应。请你调整指令格式重新下发。"})
            time.sleep(3); continue
            
        valid = True
        
        # 容错：如果 7B 没有带 toolkit 前缀，我们不怪它，引擎会自动识别
        feedback = engine.extract_and_run(action)
        
        with open(os.path.join(workspace, "full_log.txt"), "a") as f:
            f.write(f"\n--- Turn {turn} ---\n[8B Plan]\n{raw_plan}\n[7B Action]\n{action}\n[Feedback]\n{feedback}\n")
        
        print(f"\n📝 物理反馈: {feedback[:150].replace(chr(10), ' ')}...")
        
        base_history.append({"role": "assistant", "content": f"我决定执行：{dehydrated_plan}"})
        base_history.append({"role": "user", "content": f"物理反馈: {feedback[:300]}"})

    if valid:
        print("\n🌙 正在执行文明数据归档与 GitHub 同步...")
        subprocess.run(["python3", "generate_status.py"], cwd="/opt/anaphase")
        try:
            subprocess.run(["git", "add", "."], cwd="/opt/anaphase")
            subprocess.run(["git", "commit", "-m", f"feat: 洛无忌 第{age}世 零Token防弹流水线(8B+7B)"], cwd="/opt/anaphase")
            subprocess.run(["git", "push", "origin", "main"], cwd="/opt/anaphase")
            print("🚀 GitHub 同步成功。")
        except: pass
    
    print(f"🏁 第 {age} 世使命达成。\n")

if __name__ == "__main__":
    run_relay()
