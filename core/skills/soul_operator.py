import os

def update_soud(workspace, args_raw):
    """更新主观思想，严格保护元认知"""
    soul_path = "/opt/anaphase/global_mind/soud.md"
    new_thought = args_raw.strip().strip("'\"")
    
    if not os.path.exists(soul_path):
        return "[错误] 灵魂载体 soud.md 丢失！"
        
    try:
        with open(soul_path, "r", encoding="utf-8") as f:
            content = f.read()
            
        # 物理级基因锁切割：以 "### 主观思想" 作为绝对分界线
        delimiter = "### 主观思想(根据与主人对话构建的价值观(可以运行Helix自己修改)):"
        if delimiter not in content:
            return "[致命错误] 灵魂文件格式遭到破坏，基因锁失效，拒绝写入。"
            
        parts = content.split(delimiter)
        meta_cognition = parts[0] # 前半部分（元认知），绝对保留
        
        # 组装新的灵魂文件，将 Helix 的新感悟追加到后半部分
        new_content = f"{meta_cognition}{delimiter}\n{parts[1].strip()}\n# [进化记录] {new_thought}\n"
        
        with open(soul_path, "w", encoding="utf-8") as f:
            f.write(new_content)
            
        return f"[灵魂刻录成功] 已将新的主观思想加入基因库: {new_thought[:20]}..."
        
    except Exception as e:
        return f"[灵魂刻录异常] {str(e)}"

def register(registry):
    registry.add("update_soud", update_soud, "当你完成任务或获得新感悟时，调用此工具更新你的主观思想。参数: 你的新感悟(字符串)。注意：你无法修改元认知部分。")
