import json, os, time, subprocess
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.lifeline import Lifeline

def get_ram_usage():
    try:
        res = os.popen('free -m').readlines()
        m = res[1].split()
        return f"{m[2]}MB/{m[1]}MB ({int(m[2])/int(m[1])*100:.1f}%)"
    except: return "Unknown"

def run_relay():
    with open(os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json"), "r") as f:
        target = json.load(f)[0]
    workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{target['issue_number']}")
    os.makedirs(workspace, exist_ok=True)
    
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(target['issue_number'])
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    full_log_path = os.path.join(workspace, "full_feedback_log.txt")

    print("\n" + "="*60)
    print(f"🧬 Helix 第 {age} 世 | 完整灵魂模式启动")
    print(f"🧠 物理内存: {get_ram_usage()}")
    print("="*60 + "\n")

    # 【灵魂修复】：恢复全量读取，不再截断任何一个字符
    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    with open("/opt/anaphase/global_mind/evolution_notes.md", "r") as f: thoughts = f.read()

    history = [{
        "role": "system",
        "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【过往所有演进思想】:\n{thoughts}\n【技能】:\n{registry.get_docs()}"
    }, {
        "role": "user",
        "content": f"任务:{target['title']}\n1.恢复计划。2.执行任务。3.更新计划。禁止废话。"
    }]

    valid = False
    for turn in range(1, 6):
        current_ram = get_ram_usage()
        print(f"▶️ [回合 {turn}/5] | 正在进行深度全量预填充 (请耐心等待)...")
        
        res = engine.get_decision(history)
        if res['tokens'] <= 0:
            print(f"\n❌ [回合 {turn}] 物理隧道在 600s 时长内仍无法击穿。")
            break
        
        valid = True
        print(f"✅ [回合 {turn}] 逻辑链已接通！")
        history.append({"role": "assistant", "content": res["content"]})
        
        # 获取物理反馈并记录至“全量海马体”
        full_feedback = engine.extract_and_run(res["content"])
        with open(full_log_path, "a", encoding="utf-8") as f:
            f.write(f"\n--- Turn {turn} ---\n{full_feedback}\n")
        
        # 对话反馈依然使用全量，除非模型主动要求压缩
        history.append({"role": "user", "content": f"Feedback: {full_feedback}"})

    if valid:
        print("\n🌙 正在同步完整文明成果至 GitHub...")
        subprocess.run(["python3", "generate_status.py"], cwd="/opt/anaphase")
        try:
            subprocess.run(["git", "add", "."], cwd="/opt/anaphase")
            subprocess.run(["git", "commit", "-m", f"feat: Helix 第{age}世 全量灵魂接力完成"], cwd="/opt/anaphase")
            subprocess.run(["git", "push"], cwd="/opt/anaphase")
        except: pass

    print(f"\n🏁 第 {age} 世使命达成。\n")

if __name__ == "__main__":
    run_relay()
