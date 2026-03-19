def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，统计关键内存指标。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含内存统计信息的字典
    """
    import os
    
    # 检查文件是否存在
    if not os.path.exists(filepath):
        return {"error": f"文件不存在: {filepath}"}
    
    stats = {}
    
    try:
        with open(filepath, 'r') as f:
            lines = f.readlines()
            
        for line in lines:
            parts = line.split()
            if len(parts) >= 2:
                key = parts[0]
                value = int(parts[1])
                
                # 处理不同单位 (kB, MB, GB)
                unit = parts[2] if len(parts) > 2 else 'kB'
                value_in_kb = value
                
                # 转换为 MB 和 GB 以便更直观地展示
                value_in_mb = value_in_kb / 1024
                value_in_gb = value_in_mb / 1024
                
                # 存储原始值
                stats[key] = {
                    "value_kb": value_in_kb,
                    "value_mb": round(value_in_mb, 2),
                    "value_gb": round(value_in_gb, 2),
                    "unit": unit
                }
                
                # 特别关注内存相关的关键指标
                if key in ['MemTotal', 'MemFree', 'MemAvailable', 'Buffers', 'Cached', 'Slab', 'SReclaimable']:
                    stats[key]['is_critical'] = True
                else:
                    stats[key]['is_critical'] = False
                    
    except Exception as e:
        return {"error": f"读取文件时发生错误: {str(e)}"}
    
    return stats

# 示例调用
if __name__ == "__main__":
    meminfo_path = '/proc/meminfo'
    result = analyze_meminfo(meminfo_path)
    print(f"内存分析结果: {result}")