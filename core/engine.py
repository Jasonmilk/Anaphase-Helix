import re
from core.tuck_gateway import tuck_gw
from core.config import settings

class ExecutionEngine:
    def __init__(self, toolkit):
        self.toolkit = toolkit

    def get_decision(self, cognitive_kernel, target_title, history, is_first_turn=False):
        """
        is_first_turn: 只有第一轮才注入大量背景，后续轮次仅保留核心
        """
        # 极简系统指令 + 强制语法纠偏
        sys_prompt = f"""你名首席。
【认知内核】: {cognitive_kernel}
【任务】: {target_title}
【工具语法】: 
- toolkit.read_file(path, start, end)
- toolkit.propose_change(path, old_code, new_code)
- toolkit.apply_change(path, old_code, new_code)
【法则】: 只输出代码块，不准重复前世动作！
"""
        return tuck_gw.invoke_helix([{"role": "system", "content": sys_prompt}] + history, settings.TUCK_PERSONA_WORKER)

    def extract_and_run(self, content):
        code_blocks = re.findall(r'```python\n(.*?)\n```', content, re.DOTALL)
        if not code_blocks: return "❌ 错误：请输出 ```python ... ``` 指令。"
        
        code = code_blocks[0].strip()
        if "toolkit." not in code: return "❌ 拒绝：必须调用 toolkit 接口。"
            
        ldict = {"toolkit": self.toolkit, "result": None}
        try:
            # 物理执行
            exec(code.replace("toolkit.", "result = toolkit."), {"toolkit": self.toolkit}, ldict)
            return str(ldict.get("result", "Done."))
        except Exception as e:
            return f"❌ 物理报错: {str(e)}"
