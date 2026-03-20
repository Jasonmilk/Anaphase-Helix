import os

def dehydrate_experience(workspace, args_raw):
    """
    将原始对话中的关键点脱水。
    参数格式: 核心发现|主人偏好|避坑指南
    """
    lib_path = "/opt/anaphase/global_mind/experience_lib.md"
    os.makedirs(os.path.dirname(lib_path), exist_ok=True)
    
    parts = args_raw.split('|')
    if len(parts) < 3: return "[错误] 格式需为: 发现|偏好|避坑"
    
    entry = f"""
## 📅 记录时间: {os.popen('date "+%Y-%m-%d %H:%M"').read().strip()}
- **核心发现**: {parts[0].strip()}
- **主人偏好**: {parts[1].strip()}
- **避坑指南**: {parts[2].strip()}
---"""
    
    try:
        with open(lib_path, "a", encoding="utf-8") as f:
            f.write(entry)
        return "[记忆脱水成功] 关键经验已存入长期经验库(experience_lib.md)。"
    except Exception as e:
        return f"[记忆脱水失败] {str(e)}"

def register(registry):
    registry.add("dehydrate_experience", dehydrate_experience, "将本轮对话的精华脱水存入长期记忆。参数: 核心发现|主人偏好|避坑指南。")
