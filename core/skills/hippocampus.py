import os

def recall_full_feedback(workspace, args_raw):
    """
    当 Helix 觉得摘要不够完整时，调用此技能获取该任务下的所有完整物理记录。
    """
    history_path = os.path.join(workspace, "full_feedback_log.txt")
    if not os.path.exists(history_path):
        return "[错误] 尚未建立全量物理记忆。"
    
    try:
        with open(history_path, "r", encoding="utf-8") as f:
            # 返回最后 4000 字符，这是 1G 内存节点的物理极限
            return f.read()[-4000:]
    except Exception as e:
        return f"[记忆读取异常] {str(e)}"

def register(registry):
    registry.add("recall_full_feedback", recall_full_feedback, "召回被脱水的全量物理反馈。当你觉得摘要信息不足以支撑决策时使用。")
