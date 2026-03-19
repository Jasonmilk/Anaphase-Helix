import re
import os

def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，提取并统计内存相关信息。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含内存各项统计信息的字典
    """
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
            
            # 处理不同单位 (kB, MB, GB)
            unit = parts[2] if len(parts) > 2 else 'kB'
            
            # 转换为 MB 便于展示
            if unit == 'kB':
                value_mb = value / 1024
            elif unit == 'MB':
                value_mb = value
            elif unit == 'GB':
                value_mb = value * 1024
            else:
                value_mb = value
            
            stats[key] = {
                'value_kb': value,
                'value_mb': round(value_mb, 2),
                'unit': unit
            }
    
    return stats

# 示例调用
if __name__ == "__main__":
    mem_info_path = '/proc/meminfo'
    try:
        result = analyze_meminfo(mem_info_path)
        print("内存信息分析结果:")
        for key, value in result.items():
            print(f"{key}: {value['value_mb']} MB")
    except Exception as e:
        print(f"分析失败: {e}")