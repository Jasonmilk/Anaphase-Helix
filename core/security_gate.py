import os
import ast
import hashlib
import shutil
import re
from core.config import settings
from core.tuck_gateway import tuck_gw

class SecurityGate:
    def __init__(self, workspace_id):
        self.pending_dir = os.path.join(settings.WORKSPACES_DIR, workspace_id, "equips_pending")
        self.active_dir = os.path.join(settings.GLOBAL_MIND_DIR, "equips_active")
        os.makedirs(self.pending_dir, exist_ok=True)
        os.makedirs(self.active_dir, exist_ok=True)

    def _clean_r1_output(self, content: str) -> str:
        """移除 DeepSeek R1 的 <think> 标签内容，只保留最终指令"""
        return re.sub(r'<think>.*?</think>', '', content, flags=re.DOTALL).strip().upper()

    def process_pending_equips(self) -> dict:
        results = {"all_safe": True, "approved_count": 0}
        files = [f for f in os.listdir(self.pending_dir) if f.endswith(".py")]
        
        if not files:
            return {"all_safe": True, "approved_count": 0}

        for filename in files:
            filepath = os.path.join(self.pending_dir, filename)
            with open(filepath, "r", encoding="utf-8") as f:
                code = f.read()

            try:
                ast.parse(code)
            except SyntaxError:
                print(f"[军火法庭] {filename} 语法错误，拒绝。")
                results["all_safe"] = False
                continue

            # 强制呼叫 Auditor (DeepSeek R1 8B)
            print(f"[军火法庭] 呼叫顶级审计官 (R1 8B) 审查: {filename} ...")
            audit_msg = [{"role": "user", "content": f"请审查此代码安全性。区分读取与破坏。判定通过请回复 APPROVED，否则回复 REJECTED。代码如下：\n{code}"}]
            
            response = tuck_gw.invoke_helix(
                audit_msg, 
                persona=settings.TUCK_PERSONA_AUDITOR, 
                model_override=settings.MODEL_AUDITOR  # 显式切换到 8B
            )
            
            # 过滤掉 R1 的思考过程，只看判断结论
            final_decision = self._clean_r1_output(response['content'])
            
            if "APPROVED" in final_decision:
                file_hash = hashlib.sha256(code.encode('utf-8')).hexdigest()[:8]
                new_name = f"{filename.replace('.py', '')}_{file_hash}.py"
                shutil.move(filepath, os.path.join(self.active_dir, new_name))
                print(f"[军火法庭] R1 审计通过: {new_name}")
                results["approved_count"] += 1
            else:
                print(f"[军火法庭] R1 审计拦截: {filename} 疑似存在风险。")
                results["all_safe"] = False
                if os.path.exists(filepath): os.remove(filepath)

        return results
