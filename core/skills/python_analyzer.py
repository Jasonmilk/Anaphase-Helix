import ast
import os

def get_structure(workspace, args_raw):
    """利用 AST 解析 Python 文件结构，仅提取类和函数签名，极度节省 Token"""
    filepath = args_raw.strip().strip("'\"")
    target = os.path.join(workspace, filepath)
    
    if not os.path.exists(target): return f"[错误] 文件 {filepath} 不存在。"
    
    try:
        with open(target, "r", encoding="utf-8") as f:
            tree = ast.parse(f.read())
        
        structure = []
        for node in tree.body:
            if isinstance(node, ast.FunctionDef):
                structure.append(f"  - 函数: {node.name}({ast.unparse(node.args)})")
            elif isinstance(node, ast.ClassDef):
                methods = [n.name for n in node.body if isinstance(n, ast.FunctionDef)]
                structure.append(f"  - 类: {node.name} | 方法: {', '.join(methods)}")
        
        result = f"文件 {filepath} 结构分析:\n" + ("\n".join(structure) if structure else "未检测到类或函数声明。")
        return result[:1000] # 物理截断
    except Exception as e:
        return f"[分析失败] {str(e)}"

def register(registry):
    registry.add("python_analyzer", get_structure, "高效率解析Python文件结构(类/函数)，不读取全量代码。参数: 文件相对路径。")
