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
    # 1. 物理环境与任务加载
    with open(os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json"), "r") as f:
        target = json.load(f)[0]
    workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{target['issue_number']}")
    os.makedirs(workspace, exist_ok=True)
    
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(target['issue_number'])
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)

    # 2. 【核心进化】：认知蒸馏 (在加载入 Prompt 前执行)
    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    thoughts = memory.get_distilled_thoughts()
    
    print("\n" + "═"*60)
    print(f"🧬 Helix 第 {age} 世 | 认知蒸馏模式 | RAM: {get_ram_usage()}")
    print(f"📡 进化笔记已脱水，经验库已转为按需调用工具。")
    print("═"*60 + "\n")

    # 3. 构造极简高能 Prompt
    history = [{
        "role": "system",
        "content": (
            f"ID:{age}|{cycle}\n"
            f"【元认知(基因锁)】:\n{meta}\n"
            f"【思想演进(蒸馏版)】:\n{thoughts}\n"
            f"【技能定义】:\n{registry.get_docs()}\n"
            f"物理法则: 若遇到未知错误，请调用 toolkit.recall_experience 查找以往避坑指南。"
        )
    }, {
        "role": "user",
        "content": f"TASK: {target['title']}\nBODY: {target['body'][:300]}\nACTION: 直接执行。严禁任何关于背景的解释。"
    }]

    # 4. 轮回
    valid = False
    for turn in range(1, 6):
        print(f"⏳ [回合 {turn}/5] 正在通信...", end="\r")
        res = engine.get_decision(history)
        if res['tokens'] <= 0: break
        
        valid = True
        print(f"✅ [回合 {turn}] 响应成功。                     ")
        history.append({"role": "assistant", "content": res["content"]})
        
        feedback = engine.extract_and_run(res["content"])
        # 记录到全量海马体文件 (不占上下文)
        with open(os.path.join(workspace, "full_log.txt"), "a") as f:
            f.write(f"\n--- Turn {turn} ---\n{feedback}\n")
            
        history.append({"role": "user", "content": f"Feedback: {feedback[:500]}"})

    if valid:
        subprocess.run(["python3", "generate_status.py"], cwd="/opt/anaphase")
        try:
            subprocess.run(["git", "add", "."], cwd="/opt/anaphase")
            subprocess.run(["git", "commit", "-m", f"chore: Helix 第{age}世 (分级记忆模式) 认知同步"], cwd="/opt/anaphase")
            subprocess.run(["git", "push", "origin", "main"], cwd="/opt/anaphase")
        except: pass

if __name__ == "__main__":
    run_relay()
