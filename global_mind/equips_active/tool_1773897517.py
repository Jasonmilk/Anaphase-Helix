import os

# 读取 /proc/meminfo 文件
with open('/proc/meminfo', 'r') as f:
    lines = f.readlines()

# 查找包含 'MemTotal' 的行
for line in lines:
    if 'MemTotal' in line:
        # 提取数值部分（去掉 'kB:' 前缀）
        mem_total_str = line.split()[1]
        print(f"MemTotal: {mem_total_str}")
        break