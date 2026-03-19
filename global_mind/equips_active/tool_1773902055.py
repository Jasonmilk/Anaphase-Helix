def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，提取关键内存指标并生成统计报告。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含内存统计信息的字典
    """
    import os
    import json
    
    # 检查文件是否存在
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"文件不存在: {filepath}")
    
    stats = {}
    
    try:
        with open(filepath, 'r') as f:
            lines = f.readlines()
            
        # 解析内存信息
        for line in lines:
            line = line.strip()
            if ':' not in line:
                continue
                
            parts = line.split(':')
            if len(parts) != 2:
                continue
                
            key = parts[0].strip()
            value = parts[1].strip()
            
            # 移除 'kB' 后缀并转换为整数
            if value.endswith('kB'):
                value = value[:-2]
            
            try:
                value = int(value)
            except ValueError:
                continue
                
            stats[key] = value
            
        # 计算总内存和可用内存
        total_mem = stats.get('MemTotal', 0)
        available_mem = stats.get('MemAvailable', 0)
        free_mem = stats.get('MemFree', 0)
        buffers_mem = stats.get('Buffers', 0)
        cached_mem = stats.get('Cached', 0)
        
        stats['TotalMemory'] = total_mem
        stats['AvailableMemory'] = available_mem
        stats['FreeMemory'] = free_mem
        stats['BuffersMemory'] = buffers_mem
        stats['CachedMemory'] = cached_mem
        
        # 计算内存使用率
        if total_mem > 0:
            stats['MemoryUsage'] = (total_mem - available_mem) / total_mem * 100
        else:
            stats['MemoryUsage'] = 0
            
        # 计算内存碎片率
        if total_mem > 0:
            stats['MemoryFragmentation'] = (total_mem - available_mem - free_mem) / total_mem * 100
        else:
            stats['MemoryFragmentation'] = 0
            
        return stats
        
    except Exception as e:
        raise Exception(f"分析内存信息时发生错误: {str(e)}")