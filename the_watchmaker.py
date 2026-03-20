import json, os, time, subprocess
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline

def get_ram_usage():
    try:
        res = os.popen('free -m').readlines()
        m = res[1].split()
        return f"{m[2]}MB/{m[1]}MB"
    except: return "N/A"

def run_relay():
    # 1. 任务加载
    with open(os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json"), "r") as f:
        target = json.load(f)[0]
    workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{target['issue_number']}")
    os.makedirs(workspace, exist_ok=True)
    
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(target['issue_number'])
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)

    print("\n" + "═"*60)
    print(f"🧬 Helix 第 {age} 世 | 🧠 8B-R1 大脑模式 | 🦾 7B-Coder 终端模式")
    print(f"🧠 系统资源: RAM {get_ram_usage()} | 窗口保护: 8K")
    print("═"*60 + "\n")

    # 2. 灵魂与蒸馏记忆
    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    thoughts = memory.get_distilled_thoughts()
    
    # 初始思考历史
    history = [{
        "role": "system",
        "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【蒸馏思想】:\n{thoughts}\n【技能说明】:\n{registry.get_docs()}"
    }]

    # 3. 脑手联动循环
    valid = False
    for turn in range(1, 6):
        print(f"🧠 [回合 {turn}] 正在呼叫 8B-Auditor 进行深度规划...", end="\r")
        # --- 第一步：8B 大脑规划 ---
        brain_res = engine.get_decision(history + [{"role": "user", "content": f"任务:{target['title']}\n请先进行逻辑规划，使用 toolkit.scratchpad('write|计划内容') 固化思路。"}], role='brain')
        
        if brain_res['tokens'] <= 0: break
        print(f"✅ [回合 {turn}] 8B 规划完成。正在同步至海马体...")
        # 执行大脑的规划（通常是写 scratchpad）
        engine.extract_and_run(brain_res['content'])
        
        # --- 第二步：7B 手部执行 ---
        print(f"🦾 [回合 {turn}] 正在呼叫 7B-Coder 执行物理指令...", end="\r")
        hands_res = engine.get_decision(history + [
            {"role": "assistant", "content": brain_res['content']},
            {"role": "user", "content": "请根据 8B 大脑的规划，执行具体的 toolkit 工具指令。严禁废话。"}
        ], role='hands')
        
        if hands_res['tokens'] <= 0: break
        
        valid = True
        print(f"✅ [回合 {turn}] 7B 指令下达。                      ")
        history.append({"role": "assistant", "content": hands_res["content"]})
        
        # 执行物理动作
        feedback = engine.extract_and_run(hands_res["content"])
        history.append({"role": "user", "content": f"Feedback: {feedback[:1000]}"})

    if valid:
        print("\n🌙 任务完成，正在同步 GitHub...")
        subprocess.run(["python3", "generate_status.py"], cwd="/opt/anaphase")
        try:
            subprocess.run(["git", "add", "."], cwd="/opt/anaphase")
            subprocess.run(["git", "commit", "-m", f"feat: Helix 第{age}世 (8B+7B协同) 进化记录"], cwd="/opt/anaphase")
            subprocess.run(["git", "push", "origin", "main"], cwd="/opt/anaphase")
        except: pass

if __name__ == "__main__":
    run_relay()
