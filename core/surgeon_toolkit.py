import os, subprocess, difflib

class SurgeonToolkit:
    def __init__(self, workspace_dir: str):
        self.workspace = os.path.abspath(workspace_dir)

    def _secure_path(self, target_path: str) -> str:
        if os.path.isabs(target_path):
            target_path = target_path.lstrip('/')
        final_path = os.path.abspath(os.path.join(self.workspace, target_path))
        if not final_path.startswith(self.workspace):
            raise PermissionError(f"SECURITY ALERT: 越权访问 -> {target_path}")
        return final_path

    def list_dir(self, target_dir: str = ".") -> str:
        """【新增工具】列出指定目录下的文件和子目录"""
        try:
            safe_path = self._secure_path(target_dir)
            if not os.path.isdir(safe_path):
                return f"错误：'{target_dir}' 不是一个有效的目录。"
            
            items = os.listdir(safe_path)
            res = [f"--- 目录内容: {target_dir} ---"]
            for item in sorted(items):
                prefix = "[DIR] " if os.path.isdir(os.path.join(safe_path, item)) else "[FILE]"
                res.append(f"{prefix} {item}")
            return "\n".join(res)
        except Exception as e: return f"列目录失败: {e}"

    def search_code(self, keyword: str) -> str:
        try:
            result = subprocess.run(["grep", "-rnI", keyword, "."],
                cwd=self.workspace, capture_output=True, text=True, timeout=10)
            lines = result.stdout.strip().split('\n')
            if not lines or lines[0] == "": return "未找到匹配代码。"
            return "\n".join(lines[:15]) + (f"\n... (截断)" if len(lines) > 15 else "")
        except Exception: return "搜索失败"

    def read_file(self, file_path: str, start_line: int = 1, end_line: int = 100) -> str:
        try:
            safe_path = self._secure_path(file_path)
            if not os.path.exists(safe_path): return f"❌ 文件未找到: {file_path}。请先用 list_dir 确认路径！"
            with open(safe_path, 'r', encoding='utf-8', errors='ignore') as f:
                lines = f.readlines()
            start, end = max(0, start_line - 1), min(len(lines), end_line)
            return f"--- {file_path} ---\n" + "".join(f"{i+1:4d} | {lines[i]}" for i in range(start, end))
        except Exception as e: return f"读取失败: {e}"

    def propose_change(self, file_path: str, old_code: str, new_code: str) -> str:
        try:
            safe_path = self._secure_path(file_path)
            with open(safe_path, 'r', encoding='utf-8') as f: content = f.read()
            if old_code not in content: return "ERROR: old_code mismatch"
            diff = difflib.unified_diff(
                content.splitlines(), content.replace(old_code, new_code, 1).splitlines(),
                fromfile=f'a/{file_path}', tofile=f'b/{file_path}', lineterm=''
            )
            return "\n".join(list(diff))
        except Exception as e: return f"预览失败: {e}"

    def apply_change(self, file_path: str, old_code: str, new_code: str) -> str:
        try:
            safe_path = self._secure_path(file_path)
            with open(safe_path, 'r', encoding='utf-8') as f: content = f.read()
            with open(safe_path, 'w', encoding='utf-8') as f: f.write(content.replace(old_code, new_code, 1))
            return "✅ SUCCESS: 物理修改已生效"
        except Exception as e: return f"写入失败: {e}"

    def run_test(self, test_file_path: str) -> str:
        try:
            safe_path = self._secure_path(test_file_path)
            result = subprocess.run(["pytest", safe_path, "-v", "--tb=short"],
                cwd=self.workspace, capture_output=True, text=True, timeout=30)
            return f"{'✅ PASS' if result.returncode == 0 else '❌ FAIL'}\n{result.stdout[-800:]}"
        except Exception as e: return f"测试异常: {e}"

    def generate_patch(self) -> dict:
        try:
            diff = subprocess.check_output(["git", "diff"], cwd=self.workspace, text=True)
            path = os.path.abspath(os.path.join(self.workspace, "..", "fix.patch"))
            with open(path, 'w') as f: f.write(diff)
            preview = "\n".join(diff.split('\n')[:12])
            return {"path": path, "preview": preview}
        except Exception as e: return {"error": str(e)}
