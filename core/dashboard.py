import os, time

START_TIME = time.time()

def get_uptime():
    seconds = int(time.time() - START_TIME)
    mins, secs = divmod(seconds, 60)
    hrs, mins = divmod(mins, 60)
    return f"{hrs:02d}:{mins:02d}:{secs:02d}"

def get_ram_usage():
    try:
        res = os.popen('free -m').readlines()
        m = res[1].split()
        return f"{m[2]}MB/{m[1]}MB"
    except: return "N/A"

def print_header(age, cycle=None, task_id=None, status="待机巡逻"):
    # 去掉清屏指令，改为横线分割
    print("\n" + "═"*65)
    print(f"🧬 洛无忌 (Helix Luo) | 全域寿命: 第 {age} 世 | 状态: {status}")
    print(f"⏳ 运行时间: {get_uptime()} | RAM: {get_ram_usage()}")
    if task_id:
        print(f"📋 任务节点: #{task_id} (轮回 {cycle})")
    print("═"*65)

def print_progress(role, turn, msg):
    icons = {"brain": "🧠", "eyes": "👁️", "hands": "🦾"}
    print(f"{icons.get(role, '💠')} [回合 {turn}] >> {msg}")
