import os, re, json, sys
from core.config import settings
from core.tuck_gateway import tuck_gw
from core.surgeon_toolkit import SurgeonToolkit
from core.isolation_ward import IsolationWard

class Watchmaker:
    def __init__(self, issue_data):
        self.issue = issue_data
        self.ward = IsolationWard(str(issue_data['issue_number']), issue_data['repo_url'])
        self.history = []
        self.max_turns = 15
        self.total_tokens = 0

    def _build_sys_prompt(self):
        return f"""你名首席工程师。任务：修复GitHub真实Bug。

【物理工具箱】
- `toolkit.list_dir('目录')`: 查看目录下的文件。如果你找不到文件，必须先用这个！
- `toolkit.search_code('关键字')`: 搜索代码。
- `toolkit.read_file('路径', 起始行, 结束行)`: 读源码。

【生存演化SOP —— 严禁跳步】
1. [定位] 先用 `search_code` 寻找线索。
2. [确认] 如果 `search_code` 返回了路径，但你不确定结构，**必须**先用 `list_dir` 确认该目录下确实存在对应的 .py 文件。
3. [诊断] 只有确认文件存在后，才执行 `read_file`。
4. [严禁猜测] 如果 `read_file` 报错“文件未找到”，禁止再次盲目猜测！必须立刻执行 `list_dir('.')` 从根目录开始逐层核对。

【强制格式】
- 每次回复只输出一个 ```python ... ``` 代码块，不准有废话。
"""

    def solve(self):
        print(f"\n[Watchmaker] 激活侦察模式：接诊 Issue #{self.issue['issue_number']}")
        self.ward.clone_repository()
        toolkit = SurgeonToolkit(self.ward.workspace)
        self.history = [{"role": "user", "content": f"任务描述: {self.issue['title']}\n{self.issue['body']}"}]

        for turn in range(1, self.max_turns + 1):
            print(f"[轮次 {turn}] 首席正在侦察/执行...")
            res = tuck_gw.invoke_helix([{"role": "system", "content": self._build_sys_prompt()}] + self.history, settings.TUCK_PERSONA_WORKER)
            
            if res['tokens_used'] <= 0: return {"status": "FAIL", "reason": "通信异常"}
            self.total_tokens += res['tokens_used']
            self.history.append({"role": "assistant", "content": res['content']})

            code_blocks = re.findall(r'```python\n(.*?)\n```', res['content'], re.DOTALL)
            if not code_blocks:
                feedback = "❌ 警告：未发现代码块！请输出 ```python toolkit.xxx() ``` 进行探测或修复。"
            else:
                code = code_blocks[0].strip()
                feedback = self._physically_run(code, toolkit)

            print(f"[物理反馈摘要]: {feedback[:100]}...")
            self.history.append({"role": "user", "content": f"【物理反馈】:\n{feedback}"})
            
            if "凭证路径" in feedback:
                return {"status": "SUCCESS", "tokens": self.total_tokens, "path": feedback}

        return {"status": "FAIL", "reason": "轮次耗尽"}

    def _physically_run(self, code, toolkit):
        target_line = ""
        for line in code.split('\n'):
            if "toolkit." in line:
                target_line = line.strip()
                break
        if not target_line: return "❌ 错误：代码块内无有效工具调用。"

        ldict = {"toolkit": toolkit, "result": None}
        try:
            # 物理执行并抓取返回值
            exec(target_line.replace("toolkit.", "result = toolkit."), {"toolkit": toolkit}, ldict)
            res_val = ldict.get("result")
            return str(res_val) if res_val is not None else "指令已执行。"
        except Exception as e:
            return f"❌ 物理执行报错: {str(e)}"
