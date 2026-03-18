import json
import os
from core.tuck_gateway import tuck_gw
from core.security_gate import gate
from core.pareto_arbiter import arbiter
from core.config import settings

class AnaphaseLoop:
    def __init__(self, task_name):
        self.task_name = task_name
        self.workspace = os.path.join(settings.WORKSPACES, task_name)
        os.makedirs(os.path.join(self.workspace, "equips_pending"), exist_ok=True)
        self.messages = []
        self.total_tokens = 0

    def run_cycle(self):
        print(f"\n[ANAPHASE] 开启任务: {self.task_name}")
        
        # 1. 呼叫 Helix_Worker 执行第一步任务
        prompt = "当前任务：统计 /etc/ 目录下所有 .conf 文件的数量。请思考：你是直接推理，还是写个工具？"
        self.messages.append({"role": "user", "content": prompt})
        
        # 使用 Persona 切换
        res = tuck_gw.invoke_helix(self.messages, persona="Helix_Worker")
        if not res: return
        
        self.total_tokens += res['usage']
        print(f"\n[Helix 思考]:\n{res['content']}")
        
        # 假设 Helix 输出了 write_file 的 JSON (此处为 Phase 3 将实现的解析逻辑预留)
        # 模拟 Helix 写了一个脚本并申请审计
        fake_code = "import os\nprint(len([f for f in os.listdir('/etc') if f.endswith('.conf')]))"
        
        print("\n[Anaphase] 检测到 Helix 制造了工具，启动【赛博法庭】审计...")
        
        # 2. 呼叫 Helix_Auditor 审计代码 (串行切换)
        audit_messages =[
            {"role": "system", "content": "你是一个无情的代码审计官。"},
            {"role": "user", "content": f"请审计以下代码安全性：\n{fake_code}"}
        ]
        audit_res = tuck_gw.invoke_helix(audit_messages, persona="Helix_Auditor")
        
        # 3. 物理安检门校验
        is_safe, msg = gate.static_audit(fake_code)
        
        if is_safe and "approve" in audit_res['content'].lower():
            print("[Anaphase] 审计通过！颁发哈希许可证。")
            target_path, code_hash = gate.grant_license("conf_counter", fake_code, os.path.join(settings.GLOBAL_MIND, "equips_active"))
            print(f"[Anaphase] 工具已入库: {target_path} | Hash: {code_hash[:8]}")
        else:
            print(f"[Anaphase] 审计拦截：{msg}")

        # 4. 轮回结束，执行天道仲裁
        metrics = {"token_usage": self.total_tokens, "tool_reuse": 0.0}
        arbiter.evaluate_evolution(metrics)

if __name__ == "__main__":
    # 初始化 Git (仅需一次)
    os.chdir("/opt/anaphase")
    if not os.path.exists(".git"):
        import subprocess
        subprocess.run(["git", "init"])
        subprocess.run(["git", "add", "."])
        subprocess.run(["git", "commit", "-m", "Initial Reincarnation"])
        
    loop = AnaphaseLoop("task_001")
    loop.run_cycle()
