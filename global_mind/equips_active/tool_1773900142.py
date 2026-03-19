import os
import re

def analyze_meminfo():
    """
    分析 /proc/meminfo 文件，统计内存使用情况。
    返回一个包含关键内存指标字典。
    """
    meminfo_path = '/proc/meminfo'
    
    if not os.path.exists(meminfo_path):
        raise FileNotFoundError(f"文件 {meminfo_path} 不存在")
    
    stats = {}
    
    with open(meminfo_path, 'r') as f:
        for line in f:
            parts = line.split()
            if len(parts) < 2:
                continue
            
            key = parts[0]
            value_str = parts[1]
            
            # 移除行尾的 'kB' 后缀并转换为整数
            value_str = value_str.rstrip('kB')
            try:
                value = int(value_str)
            except ValueError:
                continue
            
            stats[key] = value
    
    return stats

# 示例调用
if __name__ == "__main__":
    result = analyze_meminfo()
    print("内存统计结果:", result)
    
    # 打印总内存和可用内存
    total_mem = result.get('MemTotal', 0)
    available_mem = result.get('MemAvailable', 0)
    print(f"总内存: {total_mem} kB")
    print(f"可用内存: {available_mem} kB")
    print(f"已使用内存: {total_mem - available_mem} kB")