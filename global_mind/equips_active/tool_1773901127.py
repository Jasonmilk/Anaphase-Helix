def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，统计关键内存指标。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含解析后的内存统计信息字典
    """
    import os
    
    # 检查文件是否存在
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"文件不存在: {filepath}")
    
    stats = {}
    
    try:
        with open(filepath, 'r') as f:
            for line in f:
                parts = line.split()
                if len(parts) >= 2:
                    key = parts[0]
                    value = int(parts[1])
                    
                    # 转换单位：从 kB 转换为 MB
                    value_mb = value / 1024
                    
                    # 只统计常见的内存指标
                    if key in ['MemTotal', 'MemFree', 'MemAvailable', 'Buffers', 'Cached', 
                               'Slab', 'SReclaimable', 'SwapTotal', 'SwapFree', 'SwapCached']:
                        stats[key] = {
                            'value_kb': value,
                            'value_mb': round(value_mb, 2),
                            'unit': 'MB'
                        }
    except Exception as e:
        raise RuntimeError(f"解析 {filepath} 时出错: {str(e)}")
    
    return stats

# 示例调用
if __name__ == "__main__":
    mem_path = "/proc/meminfo"
    result = analyze_meminfo(mem_path)
    print("内存统计结果:")
    for key, value in result.items():
        print(f"{key}: {value['value_mb']} {value['unit']}")