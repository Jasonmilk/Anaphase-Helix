import os
import re

def analyze_meminfo():
    """
    分析 /proc/meminfo 文件，统计内存使用情况。
    返回一个包含关键内存指标的字典。
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
            value = parts[1]
            
            # 将字节转换为 MB (保留两位小数)
            value_mb = round(int(value) / 1024 / 1024, 2)
            
            # 处理 MemFree 和 Buffers 等特殊情况，确保数值非负
            if key == 'MemFree':
                stats[key] = max(0, value_mb)
            else:
                stats[key] = value_mb
    
    return stats

# 示例调用
if __name__ == "__main__":
    try:
        memory_stats = analyze_meminfo()
        print("内存统计结果:")
        for key, value in memory_stats.items():
            print(f"{key}: {value} MB")
    except Exception as e:
        print(f"分析失败: {e}")