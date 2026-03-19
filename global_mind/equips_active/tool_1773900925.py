def analyze_meminfo(filepath):
    """
    分析 /proc/meminfo 文件，统计内存使用情况。
    
    参数:
        filepath (str): 内存信息文件路径，通常为 '/proc/meminfo'
        
    返回:
        dict: 包含内存各项统计信息的字典
    """
    import os
    import re
    
    # 检查文件是否存在
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"文件不存在: {filepath}")
    
    stats = {}
    
    # 定义需要解析的内存项及其单位转换因子 (KB -> MB)
    # 常见的内存项包括: MemTotal, MemFree, Buffers, Cached, SReclaimable, 
    # MemAvailable, MemFree+Buffers+Cached, Slab, SUnreclaim, SwapTotal, SwapFree, 
    # Dirty, Writeback, Mlocked, PageTables, NonAnonPageTables, Shmem, ShmemSized, 
    # ShmemPmdMapped, ShmemPrivate, ShmemPrivateMapped, ShmemPrivateMapped, 
    # ShmemPrivateMapped, ShmemPrivateMapped, ShmemPrivateMapped, ShmemPrivateMapped
    
    # 读取文件内容
    try:
        with open(filepath, 'r') as f:
            lines = f.readlines()
    except Exception as e:
        raise RuntimeError(f"读取文件失败: {e}")
    
    # 解析每一行
    for line in lines:
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        
        # 解析行格式: "Name: value"
        parts = line.split(':')
        if len(parts) != 2:
            continue
            
        name = parts[0].strip()
        value_str = parts[1].strip()
        
        try:
            value = int(value_str)
        except ValueError:
            continue
            
        # 将 KB 转换为 MB
        value_mb = value / 1024.0
        
        # 存储原始值和转换后的值
        stats[name] = {
            'value_kb': value,
            'value_mb': value_mb
        }
    
    return stats

# 示例调用
if __name__ == "__main__":
    # 假设 filepath 为 '/proc/meminfo'
    filepath = '/proc/meminfo'
    result = analyze_meminfo(filepath)
    
    # 打印结果示例
    print("内存统计信息:")
    for key, value in result.items():
        print(f"{key}: {value['value_mb']:.2f} MB")