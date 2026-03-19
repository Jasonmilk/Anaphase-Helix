def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，提取关键内存统计信息。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含解析后的内存统计信息字典
    """
    import os
    
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"文件不存在: {filepath}")
    
    stats = {}
    
    with open(filepath, 'r') as f:
        for line in f:
            parts = line.split()
            if len(parts) >= 2:
                key = parts[0]
                value = int(parts[1])
                stats[key] = value
    
    return stats

# 示例调用
if __name__ == "__main__":
    result = analyze_meminfo('/proc/meminfo')
    print("内存统计信息:", result)