import subprocess
import re

def analyze_meminfo():
    """
    分析 /proc/meminfo 文件，提取关键内存指标并返回统计结果。
    返回一个包含内存使用情况的字典。
    """
    try:
        # 读取 /proc/meminfo 文件
        with open('/proc/meminfo', 'r') as f:
            lines = f.readlines()
        
        stats = {}
        
        # 正则表达式匹配行格式：Name: value
        pattern = re.compile(r'^(\w+):\s+(\d+)\s+([kKMgGTPE]?[a-z]*)\s*')
        
        for line in lines:
            match = pattern.match(line.strip())
            if match:
                name = match.group(1)
                value = int(match.group(2))
                unit = match.group(3)
                
                # 根据单位转换字节数
                if unit == 'k':
                    value_bytes = value * 1024
                elif unit == 'M':
                    value_bytes = value * 1024 * 1024
                elif unit == 'G':
                    value_bytes = value * 1024 * 1024 * 1024
                else:
                    value_bytes = value
                
                stats[name] = {
                    'value': value,
                    'unit': unit,
                    'bytes': value_bytes
                }
        
        return stats

    except FileNotFoundError:
        return {"error": "无法访问 /proc/meminfo"}
    except Exception as e:
        return {"error": str(e)}

# 示例调用
if __name__ == "__main__":
    result = analyze_meminfo()
    print(result)