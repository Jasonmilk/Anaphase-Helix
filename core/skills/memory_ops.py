import os

def recall_experience(workspace, args_raw):
    """从长期经验库中检索相关知识。参数: 关键词"""
    lib_path = "/opt/anaphase/global_mind/experience_lib.md"
    if not os.path.exists(lib_path): return "尚未建立经验库。"
    
    query = args_raw.strip().lower()
    results = []
    try:
        with open(lib_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
            # 简单的关键词匹配检索，极省算力
            for i, line in enumerate(lines):
                if query in line.lower():
                    # 抓取相关行及其上下文
                    start = max(0, i-2)
                    end = min(len(lines), i+5)
                    results.append("".join(lines[start:end]))
        
        if not results: return f"未发现关于 '{query}' 的相关经验。"
        return f"--- 检索到以下相关经验 ---\n" + "\n---\n".join(results[:3])
    except Exception as e:
        return f"检索失败: {str(e)}"

def register(registry):
    registry.add("recall_experience", recall_experience, "按需检索长期经验库。参数: 关键词。建议在遇到环境障碍或逻辑困境时使用。")
