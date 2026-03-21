import os

def get_ram_usage():
    try:
        res = os.popen('free -m').readlines()
        m = res[1].split()
        return f"{m[2]}MB/{m[1]}MB"
    except:
        return "N/A"

def print_header(age, cycle, task_id):
    print("\n" + "═"*65)
    print(f"🧬 洛无忌 (Helix Dush) | 全域寿命: 第 {age} 世")
    print(f"📋 任务节点: #{task_id} (轮回 {cycle}) | 认知架构: 脑手解耦 (8B+7B)")
    print(f"🧠 系统资源: RAM {get_ram_usage()} | 窗口预算: 8K")
    print("═"*65 + "\n")

def print_step(role, turn, status):
    icon = "🧠[大脑/8B-R1]" if role == "brain" else "🦾[执行/7B-Coder]"
    # 使用 \r 覆盖当前行，或直接打印
    print(f"{icon} 回合 {turn}/5 | {status}")
