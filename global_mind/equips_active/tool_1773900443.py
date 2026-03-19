import re
import os

def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，提取关键内存统计信息。
    
    参数:
        filepath (str): 文件路径，默认为 '/proc/meminfo'
        
    返回:
        dict: 包含内存统计信息的字典
    """
    if not os.path.exists(filepath):
        return {"error": f"File not found: {filepath}"}
    
    meminfo_data = {}
    try:
        with open(filepath, 'r') as f:
            lines = f.readlines()
            
        for line in lines:
            # 解析格式: MemTotal: 16384000 kB
            match = re.match(r'^(\w+):\s+(\d+)\s+kB', line.strip())
            if match:
                key, value = match.groups()
                meminfo_data[key] = int(value)
                
        # 计算总内存（单位：kB）
        total_kb = meminfo_data.get('MemTotal', 0)
        meminfo_data['MemTotal_KB'] = total_kb
        
        # 计算可用内存（单位：kB）
        available_kb = meminfo_data.get('MemAvailable', 0)
        meminfo_data['MemAvailable_KB'] = available_kb
        
        # 计算使用率
        if total_kb > 0:
            usage_percent = (total_kb - available_kb) / total_kb * 100
            meminfo_data['MemUsage_Percent'] = round(usage_percent, 2)
        else:
            meminfo_data['MemUsage_Percent'] = 0.0
            
        return meminfo_data
        
    except Exception as e:
        return {"error": str(e)}

# 示例调用
if __name__ == "__main__":
    result = analyze_meminfo('/proc/meminfo')
    print(result)