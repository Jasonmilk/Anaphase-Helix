"""
EXTINCTION RECORD
Reason: Physical Failed: 当前系统所有网卡名称:
  - lo
  - eth0
  - docker0
Output: 当前系统所有网卡名称:
  - lo
  - eth0
  - docker0
"""

import os

def get_network_interfaces():
    """
    从 /proc/net/dev 读取并解析所有网卡名称。
    """
    interfaces = []
    
    # 检查文件是否存在
    if not os.path.exists('/proc/net/dev'):
        print("错误：/proc/net/dev 不存在。")
        return interfaces

    try:
        with open('/proc/net/dev', 'r') as f:
            lines = f.readlines()
        
        # 跳过第一行 (header)
        for line in lines[1:]:
            line = line.strip()
            if not line:
                continue
            
            # 解析行: 接口名: 接收/发送数据
            if ':' in line:
                parts = line.split(':')
                if len(parts) == 2:
                    interface_name = parts[0].strip()
                    # 过滤掉非网卡数据行 (如 lo, eth0 等)
                    # 注意: /proc/net/dev 第一列就是接口名
                    if interface_name:
                        interfaces.append(interface_name)
    
    except Exception as e:
        print(f"读取 /proc/net/dev 时出错: {e}")
        return interfaces

    return interfaces

if __name__ == "__main__":
    interfaces = get_network_interfaces()
    
    if interfaces:
        print("当前系统所有网卡名称:")
        for iface in interfaces:
            print(f"  - {iface}")
    else:
        print("未检测到网卡。")