"""
EXTINCTION RECORD
Reason: Failed: 当前系统所有网卡名称:
  - 435472071 4179492    0    0    0  
Output: 当前系统所有网卡名称:
  - 435472071 4179492    0    0    0     0          0         0 435472071 4179492    0    0    0     0       0          0
  - 783969243 1561696    0    0    0     0          0         0 151701866 1461854    0    0    0     0       0          0
  - 0       0    0    0    0     0          0         0        0       0    0    0    0     0       0          0
"""

import subprocess
import sys

def get_network_interface_names():
    """
    读取 /proc/net/dev 文件并提取所有网卡名称。
    返回一个包含网卡名称的列表。
    """
    try:
        # 读取 /proc/net/dev 文件内容
        with open('/proc/net/dev', 'r') as f:
            lines = f.readlines()
        
        network_names = []
        
        # 跳过前两行 (标题行和 IPv4 行)
        for line in lines[2:]:
            line = line.strip()
            if not line:
                continue
            
            # 检查行是否以数字开头 (表示接收字节数)，如果是则跳过
            if line[0].isdigit():
                continue
            
            # 解析行：网卡名称后跟冒号和括号，例如: eth0: 0:0, ...
            if ':' in line:
                parts = line.split(':')
                if len(parts) >= 2:
                    interface_name = parts[1].strip()
                    # 过滤掉非标准格式或空名称
                    if interface_name and not interface_name.startswith('lo'):
                        network_names.append(interface_name)
                    elif interface_name == 'lo':
                        network_names.append(interface_name)
        
        return network_names

    except FileNotFoundError:
        print("错误：无法找到 /proc/net/dev 文件。", file=sys.stderr)
        return []
    except Exception as e:
        print(f"错误：读取 /proc/net/dev 时发生异常: {e}", file=sys.stderr)
        return []

def main():
    network_names = get_network_interface_names()
    
    if not network_names:
        print("未找到任何网卡。")
        return
    
    print("当前系统所有网卡名称:")
    for name in network_names:
        print(f"  - {name}")

if __name__ == "__main__":
    main()