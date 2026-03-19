import re
from core.tuck_gateway import tuck_gw
from core.config import settings

class ExecutionEngine:
    def __init__(self, toolkit):
        self.toolkit = toolkit

    def get_decision(self, cognitive_map, target_title, target_body, history):
        # 极简且强力的指令注入
        sys_prompt = f"""你名首席工程师。
【前世认知】: {cognitive_map}
【物理限制】: 你被锁在沙盒中，严禁 `import os, sys`，严禁 `open()`。
【唯一合法指令】: 必须使用且仅能使用 `toolkit.list_dir()`, `toolkit.read_file()`, `toolkit.propose_change()`, `toolkit.apply_change()`, `toolkit.run_test()`, `toolkit.generate_patch()`。
【当前任务】: {target_title}
【要求】: 只输出代码块，不准废话。
"""
        return tuck_gw.invoke_helix([{"role": "system", "content": sys_prompt}] + history, settings.TUCK_PERSONA_WORKER)

    def extract_and_run(self, content):
        code_blocks = re.findall(r'```python\n(.*?)\n```', content, re.DOTALL)
        if not code_blocks:
            return "❌ 错误：请输出 ```python ... ``` 格式。不要在文本中解释。"
        
        code = code_blocks[0].strip()
        
        # 物理拦截：如果它在代码里写了 open 或 import，直接打断并教它怎么改
        if "open(" in code or "import os" in code:
            return "❌ 物理法则拦截：检测到非法原生 IO 操作。请删除 open/os，改用 toolkit.read_file() 或 toolkit.list_dir()。"
            
        ldict = {"toolkit": self.toolkit, "result": None}
        try:
            # 自动将 toolkit.xxx 映射为返回值
            modified_code = code.replace("toolkit.", "result = toolkit.")
            exec(modified_code, {"toolkit": self.toolkit}, ldict)
            res = ldict.get("result", "指令已执行。")
            return str(res)
        except Exception as e:
            return f"❌ 物理执行报错: {str(e)}"
