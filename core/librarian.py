import os, json, ast, time
from core.config import settings

class Librarian:
    def __init__(self):
        self.index_path = os.path.join(settings.GLOBAL_MIND_DIR, "library.json")

    def register_tool(self, file_path):
        try:
            with open(file_path, "r", encoding="utf-8") as f: code = f.read()
            tree = ast.parse(code)
            functions = []
            for node in ast.walk(tree):
                if isinstance(node, ast.FunctionDef):
                    # 只取函数名和参数，不取 docstring，极致节省 Token
                    functions.append(f"{node.name}({[arg.arg for arg in node.args.args]})")
            
            if not os.path.exists(self.index_path):
                index = {}
            else:
                with open(self.index_path, "r") as f: index = json.load(f)
            
            index[os.path.basename(file_path)] = {"f": functions, "t": int(time.time())}
            with open(self.index_path, "w") as f: json.dump(index, f)
            return True
        except: return False

    def get_knowledge_context(self):
        """生成极简索引，防止诱发 504"""
        if not os.path.exists(self.index_path): return "L:None"
        try:
            with open(self.index_path, "r") as f: index = json.load(f)
            if not index: return "L:None"
            # 只展示最新的 3 个工具
            ctx = "\n[LegacyTools]: "
            for filename, info in list(index.items())[-3:]:
                ctx += f"{filename}:{','.join(info['f'])}; "
            return ctx
        except: return "L:Err"

librarian = Librarian()
