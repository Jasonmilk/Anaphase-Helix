def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，统计内存使用情况。
    
    参数:
        filepath (str): 文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含内存各项指标的字典
    """
    import os
    import re
    
    # 检查文件是否存在
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"文件不存在: {filepath}")
    
    stats = {}
    
    # 读取文件内容
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    # 解析每一行
    for line in lines:
        parts = line.split()
        if len(parts) >= 2:
            key = parts[0]
            value = int(parts[1])
            
            # 处理不同单位 (kB, MB, GB)
            unit = parts[2] if len(parts) > 2 else 'kB'
            
            # 转换为 MB
            if unit == 'kB':
                value_mb = value / 1024
            elif unit == 'MB':
                value_mb = value
            elif unit == 'GB':
                value_mb = value * 1024
            else:
                value_mb = value / 1024
            
            # 存储原始值和转换后的值
            stats[key] = {
                'raw_value': value,
                'unit': unit,
                'value_mb': value_mb
            }
    
    return stats

# 示例调用
if __name__ == "__main__":
    meminfo_path = '/proc/meminfo'
    try:
        result = analyze_meminfo(meminfo_path)
        print("内存信息分析结果:")
        for key, value in result.items():
            print(f"{key}: {value['raw_value']} {value['unit']} ({value['value_mb']:.2f} MB)")
    except Exception as e:
        print(f"分析失败: {e}")