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

    print("\n" + "═"*60)
    print(f"🧬 Helix 第 {age} 世 | 任务 #{target['issue_number']} | 轮回 #{cycle}")
    print(f"🧠 物理内存: {get_ram_usage()}")
    print("═"*60 + "\n")

    # 1. 物理层全量读取
    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    with open("/opt/anaphase/global_mind/evolution_notes.md", "r") as f: all_thoughts = f.readlines()

    # 2. 认知压缩 (Cognitive Distillation)
    # 不删除，而是将旧记忆合并为摘要，保留最近 5 条原始内容
    if len(all_thoughts) > 10:
        summary = f"# [上古时代总结] 早期历经 {len(all_thoughts)-5} 次迭代，确立了基础物理感知与工具哲学。\n"
        recent_thoughts = "".join(all_thoughts[-5:])
        distilled_thoughts = summary + recent_thoughts
    else:
        distilled_thoughts = "".join(all_thoughts)

    history = [{
        "role": "system",
        "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【思想沉淀(高密度)】:\n{distilled_thoughts}\n【技能】:\n{registry.get_docs()}\n物理坐标: /opt/anaphase"
    }, {
        "role": "user",
        "content": f"任务: {target['title']}\n详情: {target['body']}\n\n指令: 自由思考并执行。若发现上下文丢失，请调用 recall_full_feedback 查看完整海马体。"
    }]

    valid = False
    for turn in range(1, 6):
        print(f"⏳ [回合 {turn}/5] 正在请求深度推理通道...", end="\r")
        res = engine.get_decision(history)
        if res['tokens'] <= 0: break
        
        valid = True
        print(f"✅ [回合 {turn}] 逻辑链路接通。                     ")
        history.append({"role": "assistant", "content": res["content"]})
        
        full_feedback = engine.extract_and_run(res["content"])
        
        # 记录全量反馈到海马体文件，物理上永远不缺失！
        full_log_path = os.path.join(workspace, "full_feedback_log.txt")
        with open(full_log_path, "a", encoding="utf-8") as f:
            f.write(f"\n--- Turn {turn} ---\n{full_feedback}\n")
            
        history.append({"role": "user", "content": f"【物理反馈】: {full_feedback}"})

    if valid:
        print("\n🌙 正在同步文明记录...")
        subprocess.run(["python3", "generate_status.py"], cwd="/opt/anaphase")
        try:
            subprocess.run(["git", "add", "."], cwd="/opt/anaphase")
            subprocess.run(["git", "commit", "-m", f"feat: Helix 第{age}世 (Cycle {cycle}) 认知固化"], cwd="/opt/anaphase")
            subprocess.run(["git", "push"], cwd="/opt/anaphase")
        except: pass

    print(f"🏁 第 {age} 世使命达成。\n")

if __name__ == "__main__":
    run_relay()
