import os, time, requests, json, traceback, re
from core.config import settings
from core.skill_registry import SkillRegistry
from core.engine import ExecutionEngine
from core.memory import MemoryCortex
from core.lifeline import Lifeline
from core.dashboard import print_idle_status, print_header, print_step

MIND_URL = "http://127.0.0.1:8020"

def extract_blueprint_paths(content: str):
    """
    从任务内容中提取 【前置强制要求】 后的蓝图文件路径列表。
    支持格式：
        【前置强制要求】
          - /path/to/blueprint1.md
          - /path/to/blueprint2.md
    返回路径列表。
    """
    # 查找 "【前置强制要求】" 段落
    pattern = r'【前置强制要求】\s*\n((?:\s*-\s*[^\n]+\n?)+)'
    match = re.search(pattern, content, re.MULTILINE)
    if not match:
        return []
    # 提取路径
    lines = match.group(1).strip().split('\n')
    paths = []
    for line in lines:
        # 匹配 "- /some/path"
        m = re.match(r'\s*-\s*(.+)$', line)
        if m:
            path = m.group(1).strip()
            paths.append(path)
    return paths

def execute_task(task_id, content):
    workspace = os.path.join(settings.WORKSPACES_DIR, f"task_{task_id}")
    os.makedirs(workspace, exist_ok=True)
    
    lifeline = Lifeline()
    age, cycle = lifeline.heartbeat(task_id)
    registry = SkillRegistry(workspace)
    engine = ExecutionEngine(registry)
    memory = MemoryCortex(workspace)

    print_header(age, cycle, task_id, "认知唤醒")

    with open("/opt/anaphase/global_mind/meta_cognition.md", "r") as f: meta = f.read()
    thoughts = memory.get_distilled_thoughts()
    brain_skills = registry.get_docs(mode="brain")
    hands_skills = registry.get_docs(mode="hands")

    # ========== 新增：提取并注入蓝图路径 ==========
    blueprint_paths = extract_blueprint_paths(content)
    blueprint_instruction = ""
    if blueprint_paths:
        blueprint_instruction = (
            "【前置强制蓝图】\n"
            "你必须在执行任何 toolkit 操作之前，使用 toolkit.read_file 依次读取以下蓝图文件，并严格遵守其中的命名、接口和技术约束：\n"
            + "\n".join(f"  - {p}" for p in blueprint_paths) +
            "\n\n如果蓝图中有明确的禁止行为（如禁止使用 Mutex），你必须确保最终代码不违反这些约束。\n"
        )
        # 将蓝图路径加入系统消息，让模型一开始就知道必须读取
        # 但实际还需要在 base_history 中体现
    # ===========================================

    base_history = [
        {"role": "system", "content": f"ID:{age}世|Cycle:{cycle}\n【元认知】:\n{meta}\n【思想沉淀】:\n{thoughts}"},
        {"role": "user", "content": f"任务: {content}\n请首选 toolkit.use_scratchpad('read') 恢复进度并开始工作。"}
    ]

    # 将蓝图指令插入到用户消息之前（作为系统约束），但系统消息只有一个，我们可以追加一个用户消息强调
    if blueprint_instruction:
        base_history.insert(1, {"role": "user", "content": blueprint_instruction})

    valid_interaction = False
    last_feedback = "无初始反馈。"

    # 使用配置的回合数
    for turn in range(1, settings.MAX_TURNS_PER_TASK + 1):
        print(f"\n--- [回合 {turn}/{settings.MAX_TURNS_PER_TASK}] ---")
        
        # [8B Brain]
        print_step("brain", "深层规划中...")
        brain_prompt = base_history + [{"role": "user", "content": f"可用技能: {brain_skills}\n请输出计划。"}]
        raw_plan = engine.get_decision(brain_prompt, role='brain')
        if not raw_plan: continue
            
        # [7B Hands]
        print_step("hands", "物理操作中...")
        # 强化 hands_prompt，要求严格遵守蓝图
        hands_skill_with_constraint = hands_skills + "\n【强制规则】在编写代码前，必须先用 toolkit.read_file 读取任务中的【前置强制要求】蓝图。"
        hands_prompt = base_history + [
            {"role": "system", "content": f"执行规范: {hands_skill_with_constraint}\n严禁废话，直接代码。"},
            {"role": "user", "content": f"基于计划执行动作: {raw_plan[-800:]}"},
            {"role": "assistant", "content": "toolkit."} 
        ]
        action = engine.get_decision(hands_prompt, role='hands')
        if not action: continue
            
        valid_interaction = True
        full_action = "toolkit." + action 
        last_feedback = engine.extract_and_run(full_action)
        
        # 物理留存
        with open(os.path.join(workspace, "full_log.txt"), "a") as f:
            f.write(f"\n--- Turn {turn} ---\n[Brain]\n{raw_plan}\n[Action]\n{full_action}\n[Feedback]\n{last_feedback}\n")
        
        base_history.append({"role": "assistant", "content": f"Action: {full_action}"})
        base_history.append({"role": "user", "content": f"Feedback: {last_feedback[:300]}"})
        
        if "success.txt" in last_feedback: return True, "任务闭环：目标文件已确认。"

    # 【交接棒】：增加一次容错尝试
    print_step("brain", "正在进行世代交接固化...")
    for _ in range(2): # 尝试 2 次固化
        handover_prompt = base_history + [{"role": "user", "content": "生命周期将尽，请用 toolkit.use_scratchpad('write|内容') 详细交待当前进度和下一步。"}]
        handover_res = engine.get_decision(handover_prompt, role='brain')
        if handover_res:
            engine.extract_and_run(handover_res)
            break

    summary_prompt = [{"role": "user", "content": f"总结目前物理进度(50字内):\n{last_feedback[:400]}"}]
    summary = engine.get_decision(summary_prompt, role='eyes')
    return valid_interaction, summary if summary else "进度已保存。"

def worker_loop():
    # 修复 age 获取逻辑：在循环外初始化 lifeline
    lifeline = Lifeline()
    print("🦶 洛无忌(Helix Luo) 执行触手已就绪。")
    
    while True:
        try:
            # 实时读取磁盘上的全域寿命
            with open("/opt/anaphase/global_mind/lifeline.json", 'r') as f:
                age = json.load(f).get("total_generations", 0)
            
            print_idle_status(age)
            
            resp = requests.get(f"{MIND_URL}/v1/mind/todo/pop", timeout=5)
            if resp.status_code == 200:
                data = resp.json()
                if data.get("has_task"):
                    task = data["task"]
                    success, detail = execute_task(task["id"], task["content"])
                    
                    requests.post(f"{MIND_URL}/v1/mind/report", json={
                        "task_id": task["id"], "status": "success" if success else "failed", "detail": detail
                    }, timeout=5)
                    print("\n" + "═"*65)
                    continue 
            time.sleep(5)
        except Exception as e:
            print(f"\n[⚠️ 循环异常] {str(e)}")
            time.sleep(10)

if __name__ == "__main__":
    worker_loop()
