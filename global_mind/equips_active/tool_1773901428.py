import re
import os

def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，提取关键内存指标并统计。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含内存统计信息的字典
    """
    stats = {
        'total_memory': 0,
        'available_memory': 0,
        'free_memory': 0,
        'buffers_memory': 0,
        'cached_memory': 0,
        'swap_total': 0,
        'swap_free': 0,
        'memory_percent': 0.0,
        'swap_percent': 0.0
    }
    
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"文件不存在: {filepath}")
    
    try:
        with open(filepath, 'r') as f:
            lines = f.readlines()
            
        for line in lines:
            parts = line.split()
            if len(parts) < 2:
                continue
                
            key = parts[0]
            value = int(parts[1])
            
            # 处理内存相关指标 (单位: KB)
            if key == 'MemTotal':
                stats['total_memory'] = value
            elif key == 'MemFree':
                stats['free_memory'] = value
            elif key == 'MemAvailable':
                stats['available_memory'] = value
            elif key in ['Buffers', 'Cached']:
                stats[key] = value
                
            # 处理交换空间 (Swap) 相关指标
            elif key == 'SwapTotal':
                stats['swap_total'] = value
            elif key == 'SwapFree':
                stats['swap_free'] = value
                
        # 计算百分比 (如果总内存或总交换空间不为0)
        if stats['total_memory'] > 0:
            stats['memory_percent'] = (stats['total_memory'] - stats['available_memory']) / stats['total_memory'] * 100
            
        if stats['swap_total'] > 0:
            stats['swap_percent'] = (stats['swap_total'] - stats['swap_free']) / stats['swap_total'] * 100
            
        return stats
        
    except Exception as e:
        raise RuntimeError(f"分析 {filepath} 时发生错误: {str(e)}")

# 示例调用
if __name__ == "__main__":
    meminfo_path = '/proc/meminfo'
    try:
        result = analyze_meminfo(meminfo_path)
        print("内存统计结果:")
        for key, value in result.items():
            if value > 0:
                print(f"{key}: {value} KB")
    except Exception as e:
        print(f"执行失败: {e}")