import json, os, time
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline

def run_relay():
    # 1. 任务准备
    with open(os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json"), "r") as f:
        target = json.load(f)[0]
    workspace = os.path.join(settings.WORKSPACES_DIR, f"issue_{target['issue_number']}")
    os.makedirs(workspace, exist_ok=True)
    
    # 2. 身份初始化
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(target['issue_number'])
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    
    # 3. 加载灵魂
    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    with open("/opt/anaphase/global_mind/evolution_notes.md", "r") as f: thoughts = f.read()
    
    history = [{
        "role": "system",
        "content": f"ID:{age}|{cycle}\n【元认知(不改)】:\n{meta}\n【过往思想(参考)】:\n{thoughts}\n【技能】:\n{registry.get_docs()}"
    }, {
        "role": "user",
        "content": f"任务:{target['title']}\n1.先用 toolkit.use_scratchpad('read') 恢复计划。\n2.执行任务。\n3.更新计划。禁止废话。"
    }]

    print(f"\n🧬 Helix 第{age}世 | 启动任务 #{target['issue_number']}")
    
    valid = False
    for turn in range(1, 6):
        res = engine.get_decision(history)
        if res['tokens'] <= 0: break
        
        valid = True
        print(f"[{turn}/5] 思考中...")
        history.append({"role": "assistant", "content": res["content"]})
        feedback = engine.extract_and_run(res["content"])
        history.append({"role": "user", "content": f"Feedback: {feedback[:500]}"})

    # 4. 异步灵魂提炼：仅在工作完成后
    if valid:
        print("\n🌙 进入睡眠模式，进行思想提炼...")
        final_history = history + [{"role": "user", "content": "任务已阶段性结束。请提炼一条本轮最重要的思想进化，使用 update_subjective_thought 固化。"}]
        # 此时可以手动设置 True 调用商业 API 进行高质量复盘
        final_res = engine.get_decision(final_history, use_commercial=False)
        engine.extract_and_run(final_res['content'])
        
    print(f"✅ 轮回 #{cycle} 结束.")

if __name__ == "__main__":
    run_relay()
