def analyze_meminfo():
    """
    分析 /proc/meminfo 文件，提取关键内存统计信息并返回结构化数据。
    
    返回:
        dict: 包含内存各部分统计信息的字典
    """
    import os
    import re
    
    result = {}
    
    try:
        with open('/proc/meminfo', 'r') as f:
            lines = f.readlines()
            
        for line in lines:
            # 解析每一行，格式为: 字段名: 数值(kB)
            match = re.match(r'^(\w+):\s*(\d+)', line)
            if match:
                field_name = match.group(1)
                value_kb = int(match.group(2))
                value_mb = value_kb / 1024
                
                # 只处理常见的内存字段
                if field_name in ['MemTotal', 'MemFree', 'MemAvailable', 'Buffers', 'Cached', 'Slab', 'SReclaimable', 'SwapTotal', 'SwapFree', 'SwapCached']:
                    result[field_name] = {
                        'value_kb': value_kb,
                        'value_mb': round(value_mb, 2),
                        'unit': 'MB'
                    }
                    
    except FileNotFoundError:
        result['error'] = 'Cannot access /proc/meminfo'
    except Exception as e:
        result['error'] = str(e)
        
    return result