import os

def replace_text(workspace, args_raw):
    """精确替换文件中的特定字符串，极度节省 Token"""
    # 期望参数格式: "路径|旧文本|新文本"
    parts = args_raw.split('|')
    if len(parts) != 3:
        return "[错误] 参数格式必须为 '相对路径|旧文本|新文本'。请勿遗漏管道符 |。"
        
    filepath, old_text, new_text =[p.strip().strip("'\"") for p in parts]
    target = os.path.join(workspace, filepath)
    
    if not os.path.exists(target): return f"[错误] 文件 {filepath} 不存在。"
    
    try:
        with open(target, 'r', encoding='utf-8') as f: content = f.read()
        if old_text not in content:
            return f"[错误] 未在文件中找到完全匹配的旧文本，请检查拼写。"
            
        new_content = content.replace(old_text, new_text, 1) # 只替换第一次出现
        with open(target, 'w', encoding='utf-8') as f: f.write(new_content)
        return f"[手术成功] 文件 {filepath} 的指定内容已更新。"
    except Exception as e:
        return f"[手术失败] {str(e)}"

def register(registry):
    registry.add("replace_text", replace_text, "精确修改文件。参数格式严格为: 相对路径|旧文本|新文本。例如: main.py|print('A')|print('B')")
