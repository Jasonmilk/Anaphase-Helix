import os, subprocess, difflib

class SurgeonToolkit:
    def __init__(self, workspace_dir: str):
        self.workspace = os.path.abspath(workspace_dir)
        # 文明记事本的路径 (位于 global_mind，不随沙盒销毁)
        self.notepad_path = "/opt/anaphase/global_mind/helix_notepad.md"

    def _secure_path(self, target_path: str) -> str:
        if os.path.isabs(target_path): target_path = target_path.lstrip('/')
        final_path = os.path.abspath(os.path.join(self.workspace, target_path))
        if not final_path.startswith(self.workspace):
            raise PermissionError(f"SECURITY ALERT: 越权访问 -> {target_path}")
        return final_path

    def get_system_map(self) -> str:
        """【感官】查看当前项目全景结构 (tree -L 5)"""
        try:
            res = subprocess.run(["tree", "-L", "5", self.workspace], capture_output=True, text=True)
            return res.stdout if res.stdout else "无法获取目录树。"
        except: return "系统未安装 tree 命令，请尝试 list_dir('.')"

    def read_legacy(self) -> str:
        """【记忆】阅读前世留下的文明记事本"""
        if os.path.exists(self.notepad_path):
            with open(self.notepad_path, 'r', encoding='utf-8') as f: return f.read()
        return "记事本为空。你是文明的开创者。"

    def write_legacy(self, knowledge_chunk: str) -> str:
        """【记忆】将本世的重要发现、工具用法写入记事本，传承后代"""
        try:
            with open(self.notepad_path, 'a', encoding='utf-8') as f:
                f.write(f"\n--- 世代遗产记录 ({time.strftime('%Y-%m-%d')}) ---\n{knowledge_chunk}\n")
            return "✅ 知识已成功刻入文明记事本。"
        except Exception as e: return f"写入失败: {e}"

    # ... 保留之前的 list_dir, read_file, search_code, propose, apply, run_test, generate_patch ...
    # 为了节省篇幅，此处省略，但在实际执行中必须包含
    def list_dir(self, d="."):
        try:
            s = self._secure_path(d)
            items = os.listdir(s)
            return "\n".join([f"[{'DIR' if os.path.isdir(os.path.join(s,i)) else 'FILE'}] {i}" for i in sorted(items)])
        except: return "路径错误"

    def read_file(self, p, s=1, e=100):
        try:
            with open(self._secure_path(p), 'r', errors='ignore') as f:
                l = f.readlines()
                return "".join(l[max(0,s-1):min(len(l),e)])
        except: return "读取失败"

    def search_code(self, k):
        try:
            r = subprocess.run(["grep", "-rnI", k, "."], cwd=self.workspace, capture_output=True, text=True)
            return "\n".join(r.stdout.split('\n')[:15])
        except: return "搜索失败"

    def propose_change(self, p, o, n):
        try:
            with open(self._secure_path(p), 'r') as f: c = f.read()
            if o not in c: return "ERROR: old_code mismatch"
            return "\n".join(list(difflib.unified_diff(c.splitlines(), c.replace(o,n,1).splitlines())))
        except: return "预览失败"

    def apply_change(self, p, o, n):
        try:
            with open(self._secure_path(p), 'r') as f: c = f.read()
            with open(self._secure_path(p), 'w') as f: f.write(c.replace(o,n,1))
            return "✅ 物理修改已生效"
        except: return "写入失败"

    def run_test(self, p):
        try:
            r = subprocess.run(["pytest", self._secure_path(p), "-v"], cwd=self.workspace, capture_output=True, text=True, timeout=30)
            return r.stdout[-1000:]
        except: return "测试异常"

    def generate_patch(self):
        try:
            d = subprocess.check_output(["git", "diff"], cwd=self.workspace, text=True)
            with open(os.path.join(self.workspace, "..", "fix.patch"), 'w') as f: f.write(d)
            return f"凭证路径: fix.patch\nPreview: {d[:200]}"
        except: return "生成失败"
