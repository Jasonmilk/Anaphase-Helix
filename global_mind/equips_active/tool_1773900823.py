def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，统计关键内存指标。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含内存统计信息的字典
    """
    import os
    import re
    
    stats = {}
    
    # 检查文件是否存在
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"文件不存在: {filepath}")
    
    try:
        with open(filepath, 'r') as f:
            lines = f.readlines()
        
        # 正则表达式匹配 "Name: value" 格式
        pattern = re.compile(r'^(\w+):\s*(\d+)\s*(\d+)?\s*kB\s*$')
        
        for line in lines:
            match = pattern.match(line.strip())
            if match:
                name = match.group(1)
                value = int(match.group(2))
                unit = match.group(3) or 'kB'
                
                # 转换单位为 MB (如果原始是 kB)
                if unit == 'kB':
                    value_mb = value / 1024
                else:
                    value_mb = value
                
                stats[name] = {
                    'value_kb': value,
                    'value_mb': round(value_mb, 2),
                    'unit': unit
                }
                
    except Exception as e:
        raise RuntimeError(f"读取 {filepath} 时出错: {str(e)}")
    
    return stats

# 示例调用
if __name__ == "__main__":
    mem_path = '/proc/meminfo'
    try:
        result = analyze_meminfo(mem_path)
        print("内存统计结果:")
        for key, value in result.items():
            print(f"{key}: {value['value_mb']} MB")
    except Exception as e:
        print(f"分析失败: {e}")