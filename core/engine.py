import requests, time, re

class ExecutionEngine:
    def __init__(self, registry):
        from core.config import settings
        self.registry = registry
        self.settings = settings

    def get_decision(self, history, role='hands'):
        """
        role 'brain': 使用 8B-R1 模型，高温度，适合规划。
        role 'hands': 使用 7B-Coder 模型，低温度，适合写代码和工具。
        """
        model = self.settings.MODEL_BRAIN if role == 'brain' else self.settings.MODEL_HANDS
        temp = 0.6 if role == 'brain' else 0.1
        
        # 针对 10.0.0.54 的 Tuck 路由
        url = f"{self.settings.TUCK_HOST}/v1/chat/completions"
        headers = {"Authorization": f"Bearer {self.settings.TUCK_API_KEY}", "Content-Type": "application/json"}
        
        payload = {
            "model": model,
            "messages": history,
            "temperature": temp,
            "max_tokens": 1500,
            "top_p": 0.95 if role == 'brain' else 1.0
        }

        # 针对 ARM 节点的深度接力逻辑
        for retry in range(1, 5):
            try:
                # 8B-R1 思考极慢，需给足 600s
                res = requests.post(url, headers=headers, json=payload, timeout=600)
                if res.status_code == 504:
                    print(f"\n[⚠️ {role.upper()} 接力 {retry}/4] 预填充中...")
                    time.sleep(5)
                    continue
                res.raise_for_status()
                return {"content": res.json()["choices"][0]["message"]["content"], "tokens": 1}
            except Exception as e:
                print(f"\n[Engine Error] {e}"); time.sleep(10)
        return {"content": "", "tokens": -1}

    def extract_and_run(self, content):
        # 针对 7B-Coder 强化的正则提取
        pattern = r"(?:toolkit\.)?(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "[反馈] 未检测到物理指令。"
        
        results = []
        for name, args in matches:
            if name == 'name': continue
            clean_args = re.sub(r'^\w+\s*=\s*', '', args.strip()).strip("'\"")
            # 兼容 7B-Coder 有时会多写一个右括号的情况
            clean_args = clean_args.split(')')[0]
            print(f"🛠️ 执行: {name}({clean_args[:30]}...)")
            results.append(f"【{name}反馈】: {self.registry.execute(name, clean_args)}")
        return "\n".join(results)
