import requests, time, re, os, json
from core.config import settings

class ExecutionEngine:
    def __init__(self, registry):
        self.registry = registry

    def get_decision(self, messages, role='hands'):
        if role == 'brain':
            model = getattr(settings, 'MODEL_BRAIN', "DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf")
            temp, top_p, max_tokens = 0.6, 0.95, 2048
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
                if res.status_code in [502, 504]:
                    print(f"\n[⚠️ {role.upper()} 算力接力中 ({retry}/3)]")
                    time.sleep(5); continue
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
                                    print(content, end="", flush=True)
                            except: pass
                
                print() # 换行
                if role == 'brain' and "</think>" in full_content:
                    return full_content.split("</think>")[-1].strip()
                return full_content.strip()
            except Exception as e:
                print(f"\n[❌ {role.upper()} 引擎异常] {str(e)}")
                time.sleep(5)
        return ""

    def extract_and_run(self, content):
        results =[]
        # 按行解析，完美兼容 Python 代码内的括号嵌套
        for line in content.split('\n'):
            line = line.strip().replace('```python', '').replace('```', '')
            if not line: continue
            
            match = re.search(r"(?:toolkit\.)?([a-zA-Z0-9_]+)\((.*)\)", line)
            if match:
                name = match.group(1)
                if name == 'name': continue
                
                args = match.group(2).strip()
                if args.endswith(')'): args = args[:-1]
                clean_args = re.sub(r'^\w+\s*=\s*', '', args).strip("'\"")
                
                if name in ["update_soul", "update_thought"]: name = "update_subjective_thought"
                results.append(f"【{name}】: {self.registry.execute(name, clean_args)}")
                
        if not results: return "[系统反馈] 终端未发出有效物理指令。"
        return "\n".join(results)
