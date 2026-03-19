import os
import re

def analyze_meminfo():
    """
    分析 /proc/meminfo 文件，提取关键内存指标并生成统计报告。
    
    返回:
        dict: 包含内存统计信息的字典
    """
    meminfo_path = "/proc/meminfo"
    
    if not os.path.exists(meminfo_path):
        raise FileNotFoundError(f"文件 {meminfo_path} 不存在")
    
    stats = {}
    
    with open(meminfo_path, 'r') as f:
        for line in f:
            # 解析格式: MemTotal: 16384000 kB
            match = re.match(r'^(\w+):\s+(\d+)\s+kB', line)
            if match:
                key = match.group(1)
                value = int(match.group(2))
                stats[key] = value
    
    # 计算总内存和可用内存
    total_kb = stats.get('MemTotal', 0)
    available_kb = stats.get('MemAvailable', 0)
    free_kb = stats.get('MemFree', 0)
    buffers_kb = stats.get('Buffers', 0)
    cached_kb = stats.get('Cached', 0)
    
    stats['Total_KB'] = total_kb
    stats['Available_KB'] = available_kb
    stats['Free_KB'] = free_kb
    stats['Buffers_KB'] = buffers_kb
    stats['Cached_KB'] = cached_kb
    
    # 计算使用率
    if total_kb > 0:
        usage_percent = ((total_kb - available_kb) / total_kb) * 100
        stats['Usage_Percent'] = round(usage_percent, 2)
    else:
        stats['Usage_Percent'] = 0.0
    
    # 转换为 GB
    stats['Total_GB'] = round(total_kb / 1024 / 1024, 2)
    stats['Available_GB'] = round(available_kb / 1024 / 1024, 2)
    
    return stats

# 示例调用
if __name__ == "__main__":
    result = analyze_meminfo()
    print("内存统计结果:")
    for key, value in result.items():
        print(f"{key}: {value}")