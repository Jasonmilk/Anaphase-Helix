import subprocess
import json

def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，提取关键内存指标并统计。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
    
    返回:
        dict: 包含内存统计信息的字典
    """
    if not filepath:
        filepath = '/proc/meminfo'
    
    try:
        # 读取文件内容
        with open(filepath, 'r') as f:
            lines = f.readlines()
        
        stats = {}
        
        for line in lines:
            line = line.strip()
            if not line:
                continue
            
            # 解析每一行：Key: Value
            if ':' in line:
                key, value = line.split(':', 1)
                key = key.strip()
                value = value.strip()
                
                # 移除单位 (kB 或 KB)
                if 'kB' in value or 'KB' in value:
                    value = value.replace('kB', '').replace('KB', '')
                    value = int(value)
                else:
                    value = int(value)
                
                stats[key] = value
        
        return stats
        
    except FileNotFoundError:
        return {"error": "File not found", "path": filepath}
    except Exception as e:
        return {"error": str(e), "path": filepath}

# 示例调用
if __name__ == "__main__":
    result = analyze_meminfo('/proc/meminfo')
    print(json.dumps(result, indent=2))