import os

def create_skill(workspace, args_raw):
    """
    允许智能体自主扩张技能库。
    参数格式: 技能名|Python代码
    注意: 代码中必须包含 register(registry) 函数。
    """
    try:
        parts = args_raw.split('|', 1)
        if len(parts) < 2: return "[错误] 格式需为: 技能名|代码"
        
        skill_name, code = parts[0].strip(), parts[1].strip()
        skill_path = f"/opt/anaphase/core/skills/{skill_name}.py"
        
        with open(skill_path, "w", encoding="utf-8") as f:
            f.write(code)
        
        return f"[进化成功] 新技能 {skill_name} 已物理固化，下一回合即可调用。"
    except Exception as e:
        return f"[进化失败] {str(e)}"

def register(registry):
    registry.add("create_skill", create_skill, "自主制造新技能并存入系统。参数: 技能文件名|完整Python代码。")
