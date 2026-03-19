import os, time

class MissionController:
    """任务控制器：负责熔断策略、接力计数与物理健康检查"""
    def __init__(self, max_relays=5):
        self.max_relays = max_relays
        self.relay_count_file = "/tmp/helix_relay_count" # 跨进程持久化计数

    def check_melt(self, relay_attempt, consecutive_fails):
        """三层熔断判定"""
        # 1. 接力次数熔断
        if relay_attempt > self.max_relays:
            return True, "熔断：已达到最大接力次数，防止算力空转。"
        
        # 2. 网络健康熔断
        if consecutive_fails >= 3:
            return True, "熔断：检测到持续网络异常/504，强制休眠。"
            
        return False, "HEALTHY"

    def hibernate(self, reason):
        """进入休眠状态，释放所有资源"""
        print(f"\n[Lifecycle] >>> 任务挂起并进入休眠: {reason} <<<")
        time.sleep(2) # 给 IO 一点时间

