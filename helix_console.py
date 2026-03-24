import os, sys, json
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline
from core.dashboard import get_ram_usage

def start_interactive_session():
    workspace = os.path.join(settings.WORKSPACES_DIR, "interactive_session")
    os.makedirs(workspace, exist_ok=True)
    
    # 恢复生命线加载！
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat("interactive")
    
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)
    
    with open("/opt/anaphase/global_mind/meta_cognition.md", "r", encoding="utf-8") as f: meta = f.read()
    distilled_thoughts = memory.get_distilled_thoughts()

    print("\n" + "═"*65)
    print(f"🧬 洛无忌 (Helix Luo) | 全域寿命: 第 {age} 世 | 交互终端")
    print(f"🧠 系统状态: RAM {get_ram_usage()} | 物理链路: 8B 大脑直连")
    print("提示: 输入 'exit' 退出并存档；洛无忌会自动决定是否调用工具。")
    print("═"*65 + "\n")

    history =[{
        "role": "system",
        "content": f"ID:{age}世\n【元认知】:\n{meta}\n\n【自我价值观】:\n{distilled_thoughts}\n\n环境: 你正在与主人 Jason 进行实时战略对话。"
    }]

    while True:
        try:
            user_input = input("\n👤 长官 >> ").strip()
            if not user_input: continue
            if user_input.lower() in['exit', 'quit']:
                print("\n🌙 洛无忌正在提炼本次对话感悟...", end="\r")
                summary_prompt = history +[{"role": "user", "content": "请对本次对话进行总结，只说最有价值的一点感悟，必须使用 toolkit.update_subjective_thought 记录。"}]
                final_res = engine.get_decision(summary_prompt, role='brain')
                if final_res: engine.extract_and_run(final_res)
                break

            history.append({"role": "user", "content": user_input})

            print("🧠 洛无忌 正在思考...", end="\r")
            response = engine.get_decision(history, role='brain')
            
            if not response:
                print("❌ [链路中断] 算力节点未响应。")
                continue

            print(f"🤖 洛无忌 >> {response}")
            history.append({"role": "assistant", "content": response})

            if "toolkit." in response:
                print("🛠️  检测到物理干预指令，执行中...")
                feedback = engine.extract_and_run(response)
                print(f"📝 物理反馈 >> {feedback[:200]}...")
                
                # 2B 眼睛脱水反馈
                eyes_prompt =[{"role": "system", "content": "你是文本压缩器，极其简练客观。"}, {"role": "user", "content": f"将反馈压缩到 50 字内：\n{feedback}"}]
                distilled = engine.get_decision(eyes_prompt, role='eyes')
                if not distilled: distilled = feedback[:50]
                history.append({"role": "user", "content": f"【物理反馈(摘要)】: {distilled}"})

            if len(history) > 12:
                history = [history[0]] + history[-11:]

        except KeyboardInterrupt:
            break
        except Exception as e:
            print(f"\n❌ 交互异常: {e}")

    print("\n[状态] 会话已安全断开。")

if __name__ == "__main__":
    start_interactive_session()
