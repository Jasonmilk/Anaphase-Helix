def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，统计关键内存指标。
    
    参数:
        filepath (str): 路径到 meminfo 文件，通常为 '/proc/meminfo'
        
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
            if len(parts) < 2:
                continue
            
            key = parts[0]
            value = int(parts[1])
            
            # 单位转换：kB -> MB
            if key in ['MemTotal', 'MemFree', 'MemAvailable', 'Buffers', 'Cached', 'SReclaimable', 'Slab', 'SUnreclaim', 'SwapTotal', 'SwapFree', 'SwapCached']:
                stats[key] = value / 1024.0
            elif key in ['MemTotal', 'MemFree', 'MemAvailable', 'Buffers', 'Cached', 'SReclaimable', 'Slab', 'SUnreclaim', 'SwapTotal', 'SwapFree', 'SwapCached']:
                # 确保所有内存相关字段都按 MB 输出
                stats[key] = value / 1024.0
            else:
                stats[key] = value  # 其他字段保持原值或按需转换
    
    return stats