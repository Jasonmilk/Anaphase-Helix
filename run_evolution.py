import time
from arbiter_loop import ArbiterLoop

# --- 配置区 ---
TOTAL_CYCLES = 10  # 设置你想让它自主轮回多少次
TASK = "分析 /proc/meminfo，写一个 Python 统计脚本并封装在函数内，放入 equips_pending。"

def main():
    arbiter = ArbiterLoop("task_initial")
    
    for i in range(1, TOTAL_CYCLES + 1):
        print(f"\n\n" + "="*50)
        print(f" 开始第 {i} / {TOTAL_CYCLES} 次轮回演化 ")
        print("="*50)
        
        try:
            arbiter.run_cycle(TASK)
        except Exception as e:
            print(f"本轮异常中断: {e}")
        
        # 每一轮给机器喘息 5 秒，保护 ARM CPU
        time.sleep(5)

if __name__ == "__main__":
    main()
