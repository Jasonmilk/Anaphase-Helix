import os

# 读取 /proc/meminfo 文件
with open('/proc/meminfo', 'r') as f:
    lines = f.readlines()

# 查找包含 'MemTotal' 的行
for line in lines:
    if 'MemTotal' in line:
        # 提取数值部分（去掉前缀和空格）
        parts = line.split()
        if len(parts) >= 2:
            mem_total = parts[1]
            print(f"MemTotal: {mem_total}")
        break
else:
    print("MemTotal 未找到")