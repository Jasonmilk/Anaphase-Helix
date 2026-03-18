import json
from llm_client import ask_helix

def refine_memory():
    print("[ANAPHASE 进化中心] 正在读取战地日志进行【自我复盘】...")
    
    try:
        with open("/opt/anaphase/legacy/memory.jsonl", "r", encoding="utf-8") as f:
            logs = [json.loads(line) for line in f]
        
        with open("/opt/anaphase/legacy/training.json", "r", encoding="utf-8") as f:
            current_training = f.read()
    except Exception as e:
        print(f"[ANAPHASE 错误] 读取遗产文件失败: {e}")
        return

    # 使用 4B 模型进行自我总结
    prompt = f"""
    你现在是 Helix 的【自我反思模块】。
    你刚完成了一次轮回，这是你的战地日志: {json.dumps(logs[-3:])} (仅展示最近3条)
    这是你当前的职业心法: {current_training}
    
    请根据刚才的实践，提炼出 1-2 条最核心的“避坑指南”。
    你必须且只能以 JSON 格式输出，替换原有的 domain_patches 结构。
    """
    
    messages = [
        {"role": "system", "content": "你必须只输出标准的 JSON，严禁任何文字描述。"},
        {"role": "user", "content": prompt}
    ]
    
    # 强制统一使用 4B 模型，确保在 Tuck 的超时范围内返回
    refined_data = ask_helix(messages, model_name="Qwen3.5-4B-Chat-Q4_0.gguf")
    
    if refined_data:
        # 清洗 JSON 标签
        clean_json = refined_data.replace("```json", "").replace("```", "").strip()
        try:
            # 校验是否为合法 JSON
            json.loads(clean_json) 
            with open("/opt/anaphase/legacy/training.json", "w", encoding="utf-8") as f:
                f.write(clean_json)
            print("[ANAPHASE 进化中心] 自我复盘成功！经验已注入 training.json。")
        except:
            print("[ANAPHASE 警告] 复盘数据格式错误，放弃本次进化。")
