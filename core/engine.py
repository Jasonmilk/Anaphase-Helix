import requests, time, re, os, json
from core.config import settings

class ExecutionEngine:
    def __init__(self, registry):
        self.registry = registry

    def get_decision(self, messages, role='hands'):
        if role == 'brain':
            model = getattr(settings, 'MODEL_BRAIN', "DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf")
            temp, top_p, max_tokens = 0.6, 0.95, 2048
        elif role == 'eyes':
            model = getattr(settings, 'MODEL_EYES', "Qwen3.5-2B-IQ4_NL.gguf")
            temp, top_p, max_tokens = 0.1, 1.0, 500
        else: # hands
            model = getattr(settings, 'MODEL_HANDS', "Qwen2.5.1-Coder-7B-Instruct-Q4_K_M.gguf")
            temp, top_p, max_tokens = 0.01, 1.0, 800

        url = getattr(settings, 'TUCK_HOST', "http://10.0.0.54:8686")
        if not url.endswith('/completions'): url = f"{url.rstrip('/')}/v1/chat/completions"
        headers = {"Authorization": f"Bearer {getattr(settings, 'TUCK_API_KEY', 'dummy')}", "Content-Type": "application/json"}
        
        payload = {"model": model, "messages": messages, "temperature": temp, "top_p": top_p, "max_tokens": max_tokens, "stream": True}

        for retry in range(1, 4):
            try:
                res = requests.post(url, headers=headers, json=payload, timeout=240, stream=True)
                if res.status_code in[502, 504]:
                    print(f"\n[⚠️ {role.upper()} 算力接力中 ({retry}/3)]")
                    time.sleep(5); continue
                res.raise_for_status()
                
                full_content = ""
                for line in res.iter_lines():
                    if line:
                        # 【核心修复1】：加入 errors='replace'，遇到乱码字节直接替换为 ，绝不崩溃！
                        decoded_line = line.decode('utf-8', errors='replace')
                        if decoded_line.startswith("data: ") and "[DONE]" not in decoded_line:
                            try:
                                chunk = json.loads(decoded_line[6:])
                                content = chunk["choices"][0].get("delta", {}).get("content", "")
                                if content:
                                    full_content += content
                                    if role in['brain', 'hands']:
                                        print(content, end="", flush=True)
                            except: pass
                
                if role in ['brain', 'hands']: print() 
                
                # 【核心修复2】：R1 思维兜底。如果截取后为空，强制提取思考过程！
                if role == 'brain' and "</think>" in full_content:
                    final_text = full_content.split("</think>")[-1].strip()
                    if not final_text:
                        print("⚠️ [系统] 大脑未输出明确结论，正在强行提取思维链核心...")
                        # 剔除标签，保留最后 500 字的思考结晶
                        clean_thought = full_content.replace("<think>", "").replace("</think>", "")
                        return f"[大脑深层思维推演]: {clean_thought[-500:]}"
                    return final_text
                return full_content.strip()
                
            except Exception as e:
                print(f"\n[❌ {role.upper()} 引擎异常] {str(e)}")
                time.sleep(5)
        return None

    def extract_and_run(self, content):
        pattern = r"(?:toolkit\.)?(\w+)\((.*?)\)"
        matches = re.findall(pattern, content)
        if not matches: return "[系统反馈] 终端未发出有效物理指令。"
        
        results =[]
        for name, args in matches:
            if name == 'name': continue
            clean_args = re.sub(r'^\w+\s*=\s*', '', args.strip()).strip("'\"").split(')')[0]
            if name in ["update_soul", "update_thought"]: name = "update_subjective_thought"
            results.append(f"【{name}】: {self.registry.execute(name, clean_args)}")
        return "\n".join(results)
