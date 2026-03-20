import os
import re
import ast

def create_skill(workspace, args_raw):
    """
    自主制造技能。具备自动转义修正与语法预审。
    格式: 技能名|代码内容
    """
    try:
        parts = args_raw.split('|', 1)
        if len(parts) < 2: return "[拒绝] 格式错误。需为 '技能名|代码'"
        
        name, code = parts[0].strip(), parts[1].strip()
        
        # 1. 自动转义修正：处理模型误写的 \n 字符串
        code = code.replace('\\n', '\n')
        
        # 2. 语法预审：利用 ast 模块检查代码是否合法
        try:
            ast.parse(code)
        except SyntaxError as se:
            return f"[预审拒绝] 发现语法错误: {se.msg} (行 {se.lineno})。请确保代码逻辑正确，不包含非法字符。"

        # 3. 命名校验
        if not re.match(r'^[a-z0-9_]+$', name):
            return "[拒绝] 技能名仅限小写英文、数字和下划线。"

        # 4. 物理写入
        skill_path = f"/opt/anaphase/core/skills/{name}.py"
        with open(skill_path, "w", encoding="utf-8") as f:
            f.write(code)
            
        return f"[进化成功] 技能 {name} 已通过语法预审并固化。"
    except Exception as e:
        return f"[异常] {str(e)}"

def register(registry):
    registry.add("create_skill", create_skill, "制造新技能。格式: 英文名|代码。会自动纠正换行符。")
