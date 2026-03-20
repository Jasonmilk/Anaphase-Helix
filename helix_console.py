import os
import sys
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine

def start_console():
    # 默认进入一个通用交互沙盒
    workspace = os.path.join(settings.WORKSPACES_DIR, "interactive_session")
    os.makedirs(workspace, exist_ok=True)
    
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    
    # 加载灵魂 (soud.md)
    soul_path = "/opt/anaphase/global_mind/soud.md"
    soul_content = ""
    if os.path.exists(soul_path):
        with open(soul_path, "r", encoding="utf-8") as f:
            soul_content = f.read()

    print("\n" + "="*50)
    print("🧬 Helix 自主智能体交互终端 (v1.0-Academic)")
    print("="*50)
    print("提示: 输入 'exit' 退出，输入 'clear' 清屏。")
    print(f"已加载技能: {', '.join(registry.skills.keys())}\n")

    history = [
        {"role": "system", "content": f"【你的灵魂与认知】:\n{soul_content}\n\n【环境】: 实时交互模式。你可以调用技能服务于主人的指令。"},
    ]

    while True:
        try:
            user_input = input("\n👤 长官 >> ").strip()
            if not user_input: continue
            if user_input.lower() in ['exit', 'quit']: break
            if user_input.lower() == 'clear':
                os.system('clear')
                continue

            history.append({"role": "user", "content": user_input})

            print("\n⚙️ Helix 思考中...", end="\r")
            res = engine.get_decision("", "InteractiveSession", "", history)
            
            if res['tokens_used'] <= 0:
                print("❌ 引擎响应失败，请检查网络或配置。")
                continue

            content = res['content']
            print(f"🤖 Helix >> {content}\n")
            history.append({"role": "assistant", "content": content})

            # 检测是否需要执行技能
            import re
            if "toolkit." in content:
                print("🛠️ 检测到技能请求，正在物理执行...")
                feedback = engine.extract_and_run(content)
                print(f"📝 物理反馈 >> {feedback}")
                history.append({"role": "user", "content": f"【物理反馈】: {feedback}"})
                
                # 执行完技能后，再次呼叫模型进行总结（可选）
                # 为了节省 Token，这里由用户决定是否继续。
            
            # 保持历史记录精简 (只保留最近 10 轮交互)
            if len(history) > 10:
                history = [history[0]] + history[-9:]

        except KeyboardInterrupt:
            break
        except Exception as e:
            print(f"发生错误: {e}")

    print("\n[状态] 会话已安全结束。")

if __name__ == "__main__":
    start_console()
