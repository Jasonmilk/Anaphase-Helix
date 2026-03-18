import ast
import hashlib
import os

class SecurityGate:
    """赛博军火安检门：负责脚本哈希签证与安全审计"""
    
    def __init__(self, workspace_path):
        self.workspace_path = workspace_path
        self.danger_keywords = ["os.system", "subprocess.Popen", "rm -rf", "shutil.rmtree"]

    def static_audit(self, code):
        """利用 AST 静态扫描危险函数调用"""
        try:
            tree = ast.parse(code)
            for node in ast.walk(tree):
                # 检查函数调用
                if isinstance(node, ast.Call):
                    if isinstance(node.func, ast.Attribute):
                        func_name = f"{node.func.value.id}.{node.func.attr}" if hasattr(node.func.value, 'id') else ""
                        if any(kw in func_name for kw in self.danger_keywords):
                            return False, f"检测到违禁危险调用: {func_name}"
            
            # 基础文本过滤
            if any(kw in code for kw in self.danger_keywords):
                return False, "检测到违禁危险字符串"
            
            return True, "静态扫描通过"
        except Exception as e:
            return False, f"代码语法解析失败: {e}"

    def generate_hash(self, code):
        """为脚本颁发唯一哈希签证"""
        return hashlib.sha256(code.encode('utf-8')).hexdigest()

    def grant_license(self, filename, code, target_dir):
        """正式颁发许可证并移动到现役库"""
        code_hash = self.generate_hash(code)
        target_path = os.path.join(target_dir, f"{filename}.{code_hash[:8]}.py")
        
        with open(target_path, 'w', encoding='utf-8') as f:
            f.write(code)
            
        return target_path, code_hash

gate = SecurityGate("/opt/anaphase/workspaces")
