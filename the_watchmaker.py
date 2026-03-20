import json, os, time, subprocess
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline

def run_relay():
    with open(os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json"), "r") as f:
        target = json.load(f)[0]
    workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{target['issue_number']}")
    os.makedirs(workspace, exist_ok=True)
    
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(target['issue_number'])
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    
    # 全量物理日志路径
    full_log_path = os.path.join(workspace, "full_feedback_log.txt")

    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    with open("/opt/anaphase/global_mind/evolution_notes.md", "r") as f: thoughts = f.read()
    
    print(f"\n🧬 [第{age}世] 任务#{target['issue_number']} | 300s 完整隧道开启")

    history = [{
        "role": "system",
        "content": f"ID:{age}|{cycle}\n【元认知】:\n{meta}\n【过往思想】:\n{thoughts}\n【技能】:\n{registry.get_docs()}\n注意：反馈信息若被压缩，可用 recall_full_feedback 获取全文。"
    }, {
        "role": "user",
        "content": f"Task:{target['title']}\n先读取 scratchpad。如果发现之前的反馈不完整，立即调用 recall_full_feedback。"
    }]

    valid = False
    for turn in range(1, 6):
        res = engine.get_decision(history)
        if res['tokens'] <= 0: break
        
        valid = True
        print(f"[{turn}/5] 深度预填充中 (Timeout=300s)...", end="\r")
        history.append({"role": "assistant", "content": res["content"]})
        
        # 获取完整反馈
        full_feedback = engine.extract_and_run(res["content"])
        
        # 【物理固化】：将完整、无删减的反馈写入物理日志
        with open(full_log_path, "a", encoding="utf-8") as f:
            f.write(f"\n--- Turn {turn} ---\n{full_feedback}\n")
        
        # 【摘要注入】：仅在 history 级别做摘要，保持 1G 内存流动性，但物理层保持完整
        summary = (full_feedback[:500] + "\n...[此处已压缩，完整内容已存入海马体，如有需要请 recall]...") if len(full_feedback) > 500 else full_feedback
        history.append({"role": "user", "content": f"Feedback: {summary}"})

    if valid:
        print("\n🌙 任务周期结束，同步完整演进数据...")
        subprocess.run(["python3", "generate_status.py"], cwd="/opt/anaphase")
        try:
            subprocess.run(["git", "add", "."], cwd="/opt/anaphase")
            subprocess.run(["git", "commit", "-m", f"chore: Helix 第{age}世完整物理记忆存档"], cwd="/opt/anaphase")
            subprocess.run(["git", "push"], cwd="/opt/anaphase")
        except: pass

if __name__ == "__main__":
    run_relay()
