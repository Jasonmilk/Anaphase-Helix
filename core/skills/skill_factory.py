import os
import re

def create_skill(workspace, args_raw):
    """
    自主制造技能。
    校验逻辑：禁止非英文文件名，禁止占位符内容。
    """
    try:
        parts = args_raw.split('|', 1)
        if len(parts) < 2: 
            return "[拒绝] 参数格式错误。需使用 '技能名|代码'。"
        
        skill_name = parts[0].strip()
        code = parts[1].strip()

        # 1. 严格校验技能名：仅允许英文字母、数字和下划线
        if not re.match(r'^[a-zA-Z][a-zA-Z0-9_]*$', skill_name):
            return f"[拒绝] 技能名 '{skill_name}' 不合规。必须以英文开头且无特殊字符。"

        # 2. 严格校验内容：禁止包含占位符中文
        if "完整Python代码" in code or "技能文件名" in code:
            return "[拒绝] 检测到指令说明书中的占位符。请编写真实的 Python 逻辑代码。"

        skill_path = f"/opt/anaphase/core/skills/{skill_name}.py"
        with open(skill_path, "w", encoding="utf-8") as f:
            f.write(code)
        
        return f"[执行成功] 技能 {skill_name} 已物理固化，下一回合可调用。"
    except Exception as e:
        return f"[异常] 技能生成失败: {str(e)}"

def register(registry):
    registry.add("create_skill", create_skill, "制造新技能。参数: 英文技能名|代码。")
