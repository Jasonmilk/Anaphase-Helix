import os
import sys
import json

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.append(PROJECT_ROOT)

from core.tuck_gateway import tuck_gw
from core.security_gate import gate
from core.pareto_arbiter import arbiter
from core.redis_cortex import cortex
from core.config import settings

class AnaphaseLoop:
    def __init__(self, task_name):
        self.task_name = task_name
        self.workspace = os.path.join(settings.WORKSPACES, task_name)
        os.makedirs(os.path.join(self.workspace, "equips_pending"), exist_ok=True)
        self.messages = []

    def run_cycle(self):
        print(f"\n[ANAPHASE OS] 开启轮回任务: {self.task_name}")
        
        # 1. 呼叫 Helix_Worker 执行构思
        prompt = "任务：编写一个 Python 脚本统计 /etc 目录下文件。务必通过工具实现，不要直接输出结果。"
        self.messages.append({"role": "user", "content": prompt})
        
        res = tuck_gw.invoke_helix(self.messages, persona="Helix_Worker")
        if not res: return
        
        # 记录代谢 (Redis)
        total_tokens = cortex.update_metabolism(res['usage'])
        print(f"[Anaphase] 代谢累积: {total_tokens} Tokens")
        
        # 模拟 Helix 生成的代码内容
        fake_code = "import os\nprint(len(os.listdir('/etc')))"
        
        # 2. 开启【赛博军火法庭】：群体智慧分工
        print("\n[Anaphase] 正在呼叫 Auditor 人格进行逻辑审计...")
        audit_messages = [
            {"role": "system", "content": "你是一个严厉的审计官。只回复 APPROVED 或 REJECTED。"},
            {"role": "user", "content": f"代码如下：\n{fake_code}"}
        ]
        audit_res = tuck_gw.invoke_helix(audit_messages, persona="Helix_Auditor")
        audit_verdict = audit_res['content'].strip().upper()
        print(f"[Auditor 结论]: {audit_verdict}")

        # 3. 终极逻辑闭环：Auditor 与 SecurityGate 双重通过
        is_safe, msg = gate.static_audit(fake_code)
        
        # --- 核心修正：只有当两者都通过时才发放签证 ---
        if is_safe and "APPROVED" in audit_verdict:
            print("[Anaphase] 审计通过！颁发哈希许可证。")
            target_path, code_hash = gate.grant_license(
                "task_tool", 
                fake_code, 
                os.path.join(settings.GLOBAL_MIND, "equips_active")
            )
            print(f"[Anaphase] 武器入库: {target_path}")
            
            # 4. 执行天道仲裁 (判断是否产生进化)
            metrics = {"token_usage": res['usage'], "tool_reuse": 1.0}
            arbiter.evaluate_evolution(metrics)
        else:
            print(f"[Anaphase] 拦截成功！理由: {'Auditor驳回' if 'REJECTED' in audit_verdict else msg}")
            # 审计失败，强制回滚（抹杀这一世的错误突变）
            arbiter.obliterate()

if __name__ == "__main__":
    loop = AnaphaseLoop("task_002")
    loop.run_cycle()
