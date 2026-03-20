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
    print(f"🧬 Helix-{age} | 物理资源: 4-Core CPU | 算力接力模式")
    print(f"🧠 RAM: {get_ram_usage()} | 指令修正已开启")
    print("═"*60 + "\n")

    meta = open("/opt/anaphase/global_mind/meta_cognition.md").read()
    thoughts = memory.get_distilled_thoughts()
    skill_docs = registry.get_docs()

    # 12K 预算的 Prompt 构造
    history = [{
        "role": "system",
        "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【过往沉淀】:\n{thoughts}\n【技能说明】:\n{skill_docs}\n不要使用'args:'前缀。直接填值。"
    }, {
        "role": "user",
        "content": f"Task: {target['title']}\nBody: {target['body'][:400]}\nAction: 执行并计划。禁止解释。"
    }]

    valid = False
    for turn in range(1, 6):
        # 记录重试次数，用于引擎触发 8B 模型
        retry_flag = 0
        print(f"⏳ [回合 {turn}/5] 正在通信...", end="\r")
        
        res = engine.get_decision(history, retry_count=retry_flag)
        if res['tokens'] <= 0:
            # 第一次失败，尝试开启 8B 接力
            retry_flag = 1
            res = engine.get_decision(history, retry_count=retry_flag)
            if res['tokens'] <= 0: break
        
        valid = True
        print(f"✅ [回合 {turn}] 响应成功.                     ")
        history.append({"role": "assistant", "content": res["content"]})
        
        feedback = engine.extract_and_run(res["content"])
        
        # 物理层海马体保存 (100% 完整)
        with open(os.path.join(workspace, "full_log.txt"), "a") as f:
            f.write(f"\n--- Turn {turn} ---\n{feedback}\n")
            
        history.append({"role": "user", "content": f"Feedback: {feedback[:1000]}"})

    if valid:
        subprocess.run(["python3", "generate_status.py"], cwd="/opt/anaphase")
        try:
            subprocess.run(["git", "add", "."], cwd="/opt/anaphase")
            subprocess.run(["git", "commit", "-m", f"feat: Helix 第{age}世 纠错并进化"], cwd="/opt/anaphase")
            subprocess.run(["git", "push", "origin", "main"], cwd="/opt/anaphase")
        except: pass

if __name__ == "__main__":
    run_relay()
