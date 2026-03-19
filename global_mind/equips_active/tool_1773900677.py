import os
import re

def analyze_meminfo(filepath=None):
    """
    分析 /proc/meminfo 文件，统计关键内存指标。
    
    参数:
        filepath: 可选的内存信息文件路径，默认为 '/proc/meminfo'
    
    返回:
        dict: 包含解析后的内存统计信息字典
    """
    result = {}
    
    # 如果提供了自定义路径，则使用；否则使用默认路径
    if filepath:
        meminfo_path = filepath
    else:
        meminfo_path = '/proc/meminfo'
    
    # 检查文件是否存在
    if not os.path.exists(meminfo_path):
        raise FileNotFoundError(f"内存信息文件不存在: {meminfo_path}")
    
    try:
        with open(meminfo_path, 'r') as f:
            lines = f.readlines()
        
        # 解析每一行
        for line in lines:
            # 跳过空行
            if not line.strip():
                continue
            
            # 使用正则表达式提取键值对
            # 格式示例: MemTotal:       16384000 kB
            match = re.match(r'^(\w+):\s+(\d+)\s+kB', line)
            if match:
                key = match.group(1)
                value = int(match.group(2))
                result[key] = value
        
        # 计算总内存（单位：kB）
        total_kb = result.get('MemTotal', 0)
        result['Total_KB'] = total_kb
        
        # 计算可用内存（可用内存 = 空闲 + 缓冲区 + 缓存）
        free_kb = result.get('MemFree', 0)
        buffers_kb = result.get('Buffers', 0)
        cached_kb = result.get('Cached', 0)
        available_kb = free_kb + buffers_kb + cached_kb
        result['Available_KB'] = available_kb
        
        # 计算已用内存
        used_kb = total_kb - available_kb
        result['Used_KB'] = used_kb
        
        # 计算使用率
        if total_kb > 0:
            usage_percent = (used_kb / total_kb) * 100
            result['Usage_Percent'] = round(usage_percent, 2)
        else:
            result['Usage_Percent'] = 0.0
            
    except Exception as e:
        raise RuntimeError(f"解析内存信息时发生错误: {str(e)}")
    
    return result

# 示例调用
if __name__ == "__main__":
    # 默认分析 /proc/meminfo
    stats = analyze_meminfo()
    print("内存统计信息:")
    for key, value in stats.items():
        print(f"{key}: {value}")