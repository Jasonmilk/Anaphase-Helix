#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
选题打分和合规过滤模块
功能：计算候选新闻的热度得分、相关度得分、情绪势能得分，进行合规过滤
"""

from typing import List, Dict, Any


# 敏感词列表
SENSITIVE_KEYWORDS = [
    "调查", "处罚", "反垄断", "执法", "监管重拳", "群体争议",
    "时政", "政务", "涉政", "国际冲突", "军事", "外交",
    "事故", "灾害", "犯罪", "群体事件"
]


def calculate_heat_score(candidate: Dict[str, Any]) -> float:
    """
    计算热度得分（0-10分）
    基于跨平台出现次数、热榜排名、传播属性

    Args:
        candidate: 候选新闻字典，包含platforms, rank, spread等字段

    Returns:
        热度得分（0-10）
    """
    score = 0.0

    # 1. 跨平台出现次数（每出现一个平台+0.5分，最高5分）
    platform_count = len(candidate.get('platforms', []))
    score += min(platform_count, 10) * 0.5

    # 2. 热榜排名（前10名得分高）
    rank = candidate.get('rank', 999)
    if rank <= 3:
        score += 4
    elif rank <= 10:
        score += 3
    elif rank <= 30:
        score += 2
    elif rank <= 50:
        score += 1

    # 3. 传播属性（转发/评论/点赞比率）
    spread_ratio = candidate.get('spread_ratio', 0)
    score += min(spread_ratio * 10, 2)  # 传播率每10%加0.2分，最高2分

    return min(score, 10)  # 上限10分


def calculate_relevance_score(candidate: Dict[str, Any], target_audience: str = "科技爱好者") -> float:
    """
    计算相关度得分（0-10分）
    根据目标受众和内容匹配度评估

    Args:
        candidate: 候选新闻字典
        target_audience: 目标受众群体

    Returns:
        相关度得分（0-10）
    """
    score = 5.0  # 基础分

    # 根据关键词匹配度调整
    keywords = candidate.get('keywords', '')
    content = candidate.get('content', '')

    # 科技相关关键词加分
    tech_keywords = ['AI', '人工智能', '芯片', '算法', '数据', '云计算', '区块链', '元宇宙']
    for kw in tech_keywords:
        if kw.lower() in content.lower():
            score += 1

    # 实用性内容加分
    if any(kw in content for kw in ['工具', '教程', '指南', '技巧', '方法']):
        score += 1

    return min(score, 10)


def calculate_emotion_score(candidate: Dict[str, Any]) -> float:
    """
    计算情绪势能得分（0-10分）
    评估内容的情绪张力和传播潜力

    Args:
        candidate: 候选新闻字典

    Returns:
        情绪势能得分（0-10）
    """
    score = 5.0  # 基础分
    content = candidate.get('content', '').lower()
    title = candidate.get('title', '').lower()

    # 高情绪词加分
    emotion_words = ['突破', '颠覆', '革命性', '震惊', '首次', '创纪录', '重磅', '必看']
    emotion_count = sum(1 for word in emotion_words if word in content or word in title)
    score += emotion_count * 0.5

    # 紧迫性词汇加分
    urgency_words = ['立即', '马上', '紧急', '突发', '刚刚']
    urgency_count = sum(1 for word in urgency_words if word in content or word in title)
    score += urgency_count * 0.5

    return min(score, 10)


def calculate_final_score(heat_score: float, relevance_score: float, emotion_score: float,
                         heat_weight: float = 0.5, relevance_weight: float = 0.35,
                         emotion_weight: float = 0.15) -> float:
    """
    计算最终综合得分

    Args:
        heat_score: 热度得分 0-10
        relevance_score: 相关度得分 0-10
        emotion_score: 情绪势能得分 0-10
        heat_weight: 热度权重，默认0.5
        relevance_weight: 相关度权重，默认0.35
        emotion_weight: 情绪权重，默认0.15

    Returns:
        最终得分（0-10）
    """
    return round(heat_score * heat_weight + relevance_score * relevance_weight +
                emotion_score * emotion_weight, 2)


def is_compliant(content: str, sensitive_keywords: List[str] = None) -> bool:
    """
    检查内容是否合规

    Args:
        content: 文本内容
        sensitive_keywords: 自定义敏感词列表，默认使用内置列表

    Returns:
        True表示合规，False表示不合规
    """
    if sensitive_keywords is None:
        sensitive_keywords = SENSITIVE_KEYWORDS

    content_lower = content.lower()
    for keyword in sensitive_keywords:
        if keyword.lower() in content_lower:
            return False
    return True


def is_compliant_title(title: str) -> bool:
    """
    检查标题是否合规（标题合规Gate）

    Args:
        title: 标题文本

    Returns:
        True表示合规，False表示违规
    """
    # 1. 检查煽动蛊惑词汇
    inflammatory_words = [
        "紧急通知", "速看", "看完即删", "不转后悔", "封号警告",
        "必须看", "必须转发", "不看后悔一辈子"
    ]
    for word in inflammatory_words:
        if word in title:
            return False

    # 2. 检查绝对化词汇
    absolute_words = [
        "最", "第一", "唯一", "顶级", "终极", "首创"
    ]
    # 注意：这里只检查单独的"最"、"第一"等，避免误判"最好"、"第一手"
    for word in absolute_words:
        if word in title and len(word) == len(title.strip().split()[0]):
            # 只检查标题开头的词
            if title.startswith(word) or title.startswith("最") or title.startswith("第一"):
                return False

    # 3. 检查夸大宣传词汇
    exaggeration_words = [
        "震惊", "吓傻", "惊呆", "爆笑", "笑疯了",
        "刷屏", "火爆", "疯传", "颠覆", "革命", "突破", "重磅", "里程碑"
    ]
    for word in exaggeration_words:
        if word in title:
            # 部分词汇如"突破"、"里程碑"在特定语境下可用，这里不做严格禁止
            # 只禁止明显的夸大表达
            if word in ["震惊", "吓傻", "惊呆", "爆笑", "笑疯了", "刷屏", "火爆", "疯传"]:
                return False

    # 4. 检查是否使用特殊符号对抗审核
    # 检查过多的点号、空格等
    if title.count("。") > 2 or title.count(".") > 2:
        return False

    # 5. 检查标题是否包含敏感词
    if not is_compliant(title):
        return False

    return True


def generate_click_reason(target_audience: str, change: str, urgency: str) -> str:
    """
    生成点击理由

    Args:
        target_audience: 目标受众
        change: 发生的变化
        urgency: 为什么现在必须知道

    Returns:
        点击理由字符串
    """
    return f"对{target_audience}，{change}，{urgency}"


def score_title(title: str) -> Dict[str, float]:
    """
    对标题进行评分

    Args:
        title: 标题文本

    Returns:
        评分字典，包含各维度得分和总分
    """
    # 1. 明确度（0-3分）
    clarity = 3.0  # 默认基础分

    # 简单的明确度判断：标题长度适中、不包含模糊词汇
    if len(title) < 10 or len(title) > 40:
        clarity -= 1.0

    # 包含具体数字加分
    if any(c.isdigit() for c in title):
        clarity += 0.5

    clarity = max(0, min(3, clarity))

    # 2. 利益点（0-3分）
    benefit = 2.0  # 默认基础分

    benefit_words = ["节省", "提升", "降低", "告别", "必备", "神器", "效率", "免费"]
    for word in benefit_words:
        if word in title:
            benefit += 0.5
            break

    benefit = max(0, min(3, benefit))

    # 3. 紧迫性（0-2分）
    urgency = 1.0  # 默认基础分

    urgency_words = ["马上", "立即", "现在", "今天", "限时", "仅剩", "刚刚", "即将"]
    for word in urgency_words:
        if word in title:
            urgency += 0.5
            break

    urgency = max(0, min(2, urgency))

    # 4. 合规性（0-2分）
    if is_compliant_title(title):
        compliance = 2.0
    else:
        compliance = 0.0

    total_score = clarity + benefit + urgency + compliance

    return {
        "clarity": clarity,
        "benefit": benefit,
        "urgency": urgency,
        "compliance": compliance,
        "total": total_score
    }


def is_compliant(content: str, sensitive_keywords: List[str] = None) -> bool:
    """
    检查内容是否合规（与上面的函数重复，保留兼容性）

    Args:
        content: 文本内容
        sensitive_keywords: 自定义敏感词列表，默认使用内置列表

    Returns:
        True表示合规，False表示不合规
    """
    if sensitive_keywords is None:
        sensitive_keywords = SENSITIVE_KEYWORDS

    content_lower = content.lower()
    for keyword in sensitive_keywords:
        if keyword.lower() in content_lower:
            return False
    return True


def filter_candidates(candidates: List[Dict[str, Any]],
                     min_score: float = 5.0,
                     enforce_compliance: bool = True) -> List[Dict[str, Any]]:
    """
    过滤候选新闻并打分

    Args:
        candidates: 候选新闻列表
        min_score: 最低得分阈值，低于此分数的候选将被过滤
        enforce_compliance: 是否强制执行合规检查

    Returns:
        过滤并打分后的候选列表，按最终得分降序排序
    """
    filtered = []

    for candidate in candidates:
        # 合规检查
        content = candidate.get('content', '') + ' ' + candidate.get('title', '')
        if enforce_compliance and not is_compliant(content):
            continue

        # 计算各项得分
        heat = calculate_heat_score(candidate)
        relevance = calculate_relevance_score(candidate)
        emotion = calculate_emotion_score(candidate)

        # 计算最终得分
        final_score = calculate_final_score(heat, relevance, emotion)

        # 低于最低分数阈值则过滤
        if final_score < min_score:
            continue

        # 添加得分到候选
        candidate['heat_score'] = heat
        candidate['relevance_score'] = relevance
        candidate['emotion_score'] = emotion
        candidate['final_score'] = final_score

        filtered.append(candidate)

    # 按最终得分降序排序
    filtered.sort(key=lambda x: x['final_score'], reverse=True)

    return filtered


def print_score_breakdown(candidate: Dict[str, Any]):
    """
    打印候选新闻的得分明细

    Args:
        candidate: 候选新闻字典，需包含各项得分
    """
    print(f"标题: {candidate.get('title', 'N/A')}")
    print(f"  热度得分: {candidate.get('heat_score', 0):.2f}")
    print(f"  相关度得分: {candidate.get('relevance_score', 0):.2f}")
    print(f"  情绪得分: {candidate.get('emotion_score', 0):.2f}")
    print(f"  最终得分: {candidate.get('final_score', 0):.2f}")
    print()


if __name__ == "__main__":
    # 测试用例
    test_candidates = [
        {
            "title": "苹果发布革命性AI芯片，震惊科技界",
            "platforms": ["微博", "知乎", "B站", "抖音", "公众号"],
            "rank": 3,
            "spread_ratio": 0.9,
            "content": "苹果刚刚发布了全新的AI芯片，性能突破传统限制，这是首次在移动端实现如此强大的AI能力。",
            "entity": "Apple",
            "action": "发布",
            "keywords": "AI,芯片",
            "url": "https://example.com/news/1"
        },
        {
            "title": "某地方政府加强执法监管",
            "platforms": ["微博"],
            "rank": 20,
            "spread_ratio": 0.3,
            "content": "地方政府表示将加强执法监管力度，严肃处理相关违规行为",
            "entity": "政府",
            "action": "监管",
            "keywords": "执法,监管",
            "url": "https://example.com/news/2"
        }
    ]

    # 过滤并打分
    filtered = filter_candidates(test_candidates)

    # 打印结果
    print(f"筛选结果: {len(filtered)} 条")
    for candidate in filtered:
        print_score_breakdown(candidate)
