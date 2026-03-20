print("开始模块加载自检...")
try:
    print("1. 尝试导入 core.config...", end="")
    from core.config import settings
    print(" 成功")
    
    print("2. 尝试导入 core.lifeline...", end="")
    from core.lifeline import Lifeline
    print(" 成功")

    print("3. 尝试导入 core.skill_registry...", end="")
    from core.skill_registry import SkillRegistry
    print(" 成功")

    print("4. 尝试导入 core.engine...", end="")
    from core.engine import ExecutionEngine
    print(" 成功")

    print("5. 尝试导入 core.memory...", end="")
    from core.memory import MemoryCortex
    print(" 成功")
    
    print("\n[恭喜] 所有模块加载成功！说明环境和代码结构是通的。")
except Exception as e:
    print(f"\n❌ [导入失败] 崩溃点在上面显示的模块中。错误原因: {e}")
    import traceback
    traceback.print_exc()
