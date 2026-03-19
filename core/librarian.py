import os
import json
import ast
from core.config import settings

class Librarian:
    """文明馆长：负责知识的提取、归档与传承"""
    
    def __init__(self):
        self.index_path = os.path.join(settings.GLOBAL_MIND_DIR, "library.json")
        self.equips_dir = os.path.join(settings.GLOBAL_MIND_DIR, "equips_active")

    def register_tool(self, file_path):
        """解析 Python 脚本，提取函数签名和注释，存入索引"""
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                code = f.read()
            
            tree = ast.parse(code)
            filename = os.path.basename(file_path)
            
            # 提取函数定义
            functions = []
            for node in ast.walk(tree):
                if isinstance(node, ast.FunctionDef):
                    docstring = ast.get_docstring(node) or "无描述"
                    functions.append({
                        "name": node.name,
                        "args": [arg.arg for arg in node.args.args],
                        "doc": docstring
                    })

            # 更新索引
            with open(self.index_path, "r") as f:
                index = json.load(f)
            
            index[filename] = {
                "functions": functions,
                "path": file_path,
                "added_time": os.path.getmtime(file_path)
            }
            
            with open(self.index_path, "w", encoding="utf-8") as f:
                json.dump(index, f, ensure_ascii=False, indent=2)
            
            return True
        except Exception as e:
            print(f"[Librarian Error] 解析工具失败: {e}")
            return False

    def get_knowledge_context(self):
        """生成『文明遗产』描述，用于注入下一次任务的 Prompt"""
        if not os.path.exists(self.index_path): return "无 (文明初创，需自行探索)"
        
        with open(self.index_path, "r") as f:
            index = json.load(f)
        
        if not index: return "无 (武器库暂空)"
        
        context = "\n【文明遗产：以下为你前辈留下的现成工具，请优先调用以节省 Token】\n"
        for filename, info in index.items():
            for func in info['functions']:
                context += f"- 工具文件: {filename} | 函数: {func['name']}({', '.join(func['args'])}) | 作用: {func['doc']}\n"
        return context

librarian = Librarian()
