import os
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine

def start_console():
    workspace = os.path.join(settings.WORKSPACES_DIR, "interactive_session")
    os.makedirs(workspace, exist_ok=True)
    
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    
    soul_path = "/opt/anaphase/global_mind/soud.md"
    lib_path = "/opt/anaphase/global_mind/experience_lib.md"
    
    def get_context():
        content = ""
        if os.path.exists(soul_path):
            with open(soul_path, "r") as f: content += f"【灵魂】:\n{f.read()}\n"
        if os.path.exists(lib_path):
            # 只读取最近的 500 字符经验，防止爆内存
            with open(lib_path, "r") as f: 
                f.seek(0, os.SEEK_END)
                size = f.tell()
                f.seek(max(0, size - 500))
                content += f"【近期脱水经验】:\n...{f.read()}\n"
        return content

    print("\n" + "="*50)
    print("🧬 Helix 自主智能体交互终端 (v1.1-Dehydrate)")
    print("="*50)

    history = [{"role": "system", "content": f"{get_context()}\n环境: 交互模式。退出前请务必使用 dehydrate_experience 总结。"}]

    while True:
        try:
            user_input = input("\n👤 长官 >> ").strip()
            if not user_input: continue
            
            # 当用户想要退出时，强制 Helix 进行最后一次脱水总结
            if user_input.lower() in ['exit', 'quit']:
                print("\n🧪 Helix 正在进行认知脱水与结算...")
                res = engine.get_decision("", "FinalReview", "", history + [{"role": "user", "content": "请对本次对话进行脱水总结并调用技能存储，然后道别。"}])
                if "toolkit." in res['content']:
                    print(engine.extract_and_run(res['content']))
                print(f"🤖 Helix >> {res['content'].split('toolkit')[0]}")
                break

            history.append({"role": "user", "content": user_input})
            print("⚙️ Helix 思考中...", end="\r")
            res = engine.get_decision("", "Interactive", "", history)
            
            content = res['content']
            print(f"🤖 Helix >> {content}")
            history.append({"role": "assistant", "content": content})

            if "toolkit." in content:
                feedback = engine.extract_and_run(content)
                print(f"📝 物理反馈 >> {feedback}")
                history.append({"role": "user", "content": f"【反馈】: {feedback}"})

            if len(history) > 10: history = [history[0]] + history[-9:]

        except KeyboardInterrupt: break
    print("\n[状态] 会话已安全结束。")

if __name__ == "__main__":
    start_console()
