import requests, time, re, os, json
from core.config import settings

class ExecutionEngine:
    def __init__(self, registry):
        self.registry = registry

    def get_decision(self, messages, role='hands'):
        # 1. 角色分配参数
        if role == 'brain':
            model = getattr(settings, 'MODEL_BRAIN', "DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf")
            temp, top_p, max_tokens = 0.6, 0.95, 2048
        elif role == 'eyes':
            model = getattr(settings, 'MODEL_EYES', "Qwen3.5-2B-IQ4_NL.gguf")
            temp, top_p, max_tokens = 0.1, 1.0, 500
        else: # hands
            model = getattr(settings, 'MODEL_HANDS', "Qwen2.5.1-Coder-7B-Instruct-Q4_K_M.gguf")
            temp, top_p, max_tokens = 0.05, 1.0, 800

        url = getattr(settings, 'TUCK_HOST', "http://10.0.0.54:8686")
        if not url.endswith('/completions'): url = f"{url.rstrip('/')}/v1/chat/completions"
        headers = {"Authorization": f"Bearer {getattr(settings, 'TUCK_API_KEY', 'dummy')}", "Content-Type": "application/json"}
        
        payload = {"model": model, "messages": messages, "temperature": temp, "top_p": top_p, "max_tokens": max_tokens, "stream": True}

        for retry in range(1, 4):
            try:
                res = requests.post(url, headers=headers, json=payload, timeout=120, stream=True)
                
                # 【防盲探针】：精准识别 404 模型未找到错误
                if res.status_code == 404:
                    print(f"\n[❌ 404 寻址失败] Tuck 网关中不存在模型: '{model}'。请检查 .env 配置！")
                    return None
                    
                if res.status_code in[502, 504]:
                    print(f"\n[⚠️ {role.upper()} 算力预热] 正在接力 ({retry}/3)...")
                    time.sleep(5)
                    continue
                    
                res.raise_for_status()
                
                full_content = ""
                for line in res.iter_lines():
                    if line:
                        decoded_line = line.decode('utf-8')
                        if decoded_line.startswith("data: ") and "[DONE]" not in decoded_line:
                            try:
                                chunk = json.loads(decoded_line[6:])
                                content = chunk["choices"][0].get("delta", {}).get("content", "")
                                if content:
                                    full_content += content
                                    if len(full_content) % 100 == 0: print(".", end="", flush=True)
                            except: pass
                print(" [接收完毕]")
                
                if role == 'brain' and "</think>" in full_content:
                    return full_content.split("</think>")[-1].strip()
                return full_content
            except Exception as e:
                print(f"\n[❌ {role.upper()} 引擎异常] {str(e)}")
                time.sleep(5)
        return None

    def extract_and_run(self, content):
        pattern = r"(?:toolkit\.)?(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "[系统反馈] 7B 终端未发出有效物理指令。"
        
        results =[]
        for name, args in matches:
            if name == 'name': continue
            clean_args = re.sub(r'^\w+\s*=\s*', '', args.strip()).strip("'\"").split(')')[0]
            if name in["update_soul", "update_thought"]: name = "update_subjective_thought"
            results.append(f"【{name}】: {self.registry.execute(name, clean_args)}")
        return "\n".join(results)
