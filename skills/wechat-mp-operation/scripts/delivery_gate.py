#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
交付质量检查模块
功能：输出前进行多维度质量检查（结构、事实、HTML合规、查重）
"""

import re
from typing import Dict, List, Any, Tuple
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from duplicate_checker import DuplicateChecker


def check_structure(html_content: str, content_data: Dict[str, str]) -> Dict[str, Any]:
    """
    检查HTML结构完整性

    Args:
        html_content: HTML内容
        content_data: 内容数据字典

    Returns:
        检查结果字典
    """
    errors = []
    warnings = []

    # 检查必需的HTML标签
    required_tags = ['<!DOCTYPE html>', '<html', '<head>', '<body>']
    for tag in required_tags:
        if tag not in html_content:
            errors.append(f"缺少必需标签: {tag}")

    # 检查内容数据是否完整
    required_fields = ['headline_title', 'headline_content', 'sub_news']
    for field in required_fields:
        if not content_data.get(field):
            errors.append(f"内容数据缺少必需字段: {field}")

    # 检查HTML长度
    html_length = len(html_content)
    if html_length < 1000:
        warnings.append(f"HTML内容过短（{html_length}字符），可能不完整")

    # 检查关键模块是否包含
    key_sections = ['头条', '行业要闻', '参考来源']
    for section in key_sections:
        if section not in html_content:
            warnings.append(f"缺少关键模块: {section}")

    passed = len(errors) == 0

    return {
        'passed': passed,
        'errors': errors,
        'warnings': warnings
    }


def check_facts(content_data: Dict[str, str]) -> Dict[str, Any]:
    """
    检查事实一致性

    Args:
        content_data: 内容数据字典

    Returns:
        检查结果字典
    """
    errors = []
    warnings = []

    headline_title = content_data.get('headline_title', '')
    headline_content = content_data.get('headline_content', '')
    publish_date = content_data.get('publish_date', '')

    # 检查标题是否为空
    if not headline_title or headline_title.strip() == '':
        errors.append("头条标题为空")

    # 检查内容是否为空
    if not headline_content or headline_content.strip() == '':
        errors.append("头条内容为空")

    # 检查日期格式
    date_pattern = re.compile(r'^\d{4}-\d{2}-\d{2}$')
    if not date_pattern.match(publish_date):
        warnings.append(f"发布日期格式可能不正确: {publish_date}")

    # 检查内容长度
    if len(headline_content) < 200:
        warnings.append("头条内容过短，建议至少200字")

    # 检查是否有明显的待替换标记
    if '{{' in headline_content or '}}' in headline_content:
        errors.append("头条内容包含未替换的占位符")

    passed = len(errors) == 0

    return {
        'passed': passed,
        'errors': errors,
        'warnings': warnings
    }


def check_html_compliance(html_content: str) -> Dict[str, Any]:
    """
    检查HTML合规性

    Args:
        html_content: HTML内容

    Returns:
        检查结果字典
    """
    errors = []
    warnings = []

    # 检查标签闭合
    unclosed_tags = re.findall(r'<(?!br|hr|img|input|meta|link)([^>]+)(?<!/)>', html_content)
    if unclosed_tags:
        warnings.append(f"可能存在未闭合的标签: {unclosed_tags[:5]}")

    # 检查图片标签
    img_tags = re.findall(r'<img[^>]*>', html_content)
    for img in img_tags:
        if 'src=' not in img:
            errors.append("存在缺少src属性的img标签")
        if 'alt=' not in img:
            warnings.append("图片缺少alt属性")

    # 检查CSS样式
    if '<style' in html_content:
        # 检查是否有内联样式（推荐）
        if 'style="' in html_content or "style='" in html_content:
            pass  # 有内联样式，良好
        else:
            warnings.append("建议使用内联样式以兼容微信公众号")

    # 检查是否有短链接
    short_link_pattern = re.compile(r'bit\.ly|t\.co|goo\.gl')
    if short_link_pattern.search(html_content):
        errors.append("HTML中包含短链接，建议使用image_url_fixer修复")

    # 检查是否有JavaScript（微信公众号不支持）
    if '<script' in html_content or 'javascript:' in html_content:
        errors.append("HTML中包含JavaScript，微信公众号不支持")

    # 检查是否使用了外部CSS（微信公众号不支持）
    if '<link rel="stylesheet"' in html_content:
        errors.append("HTML中包含外部CSS链接，微信公众号不支持")

    passed = len(errors) == 0

    return {
        'passed': passed,
        'errors': errors,
        'warnings': warnings
    }


def check_duplicate_content(html_content: str, threshold: float = 0.8) -> Dict[str, Any]:
    """
    检查内容重复性（基于文本相似度）

    Args:
        html_content: HTML内容
        threshold: 相似度阈值，超过此值则警告

    Returns:
        检查结果字典
    """
    warnings = []

    # 提取纯文本
    text = re.sub(r'<[^>]+>', ' ', html_content)
    text = ' '.join(text.split())

    # 简单的段落重复检查
    paragraphs = text.split('\n')
    paragraphs = [p.strip() for p in paragraphs if p.strip()]

    # 检查连续重复的段落
    for i in range(len(paragraphs) - 1):
        if paragraphs[i] == paragraphs[i + 1]:
            warnings.append(f"发现连续重复段落: {paragraphs[i][:50]}...")

    # 检查过于相似的内容
    if len(paragraphs) > 2:
        for i in range(len(paragraphs) - 1):
            for j in range(i + 1, len(paragraphs)):
                # 简单的相似度计算（基于字符重叠）
                p1, p2 = paragraphs[i], paragraphs[j]
                overlap = len(set(p1) & set(p2))
                similarity = overlap / max(len(p1), len(p2)) if max(len(p1), len(p2)) > 0 else 0

                if similarity > threshold and len(p1) > 50:
                    warnings.append(f"发现相似度过高的段落 ({similarity:.2f})")

    passed = len(warnings) == 0

    return {
        'passed': passed,
        'errors': [],
        'warnings': warnings
    }


def check_history_duplicate(entity: str, action: str, keywords: str,
                            db_path: str = None, days: int = 5) -> Dict[str, Any]:
    """
    检查与历史库的重复

    Args:
        entity: 主体
        action: 动作
        keywords: 关键词
        db_path: 数据库路径
        days: 查重天数窗口

    Returns:
        检查结果字典
    """
    errors = []

    try:
        checker = DuplicateChecker(db_path)
        is_dup = checker.is_duplicate(entity, action, keywords, days)

        if is_dup:
            errors.append(f"与历史库重复: {entity} {action} {keywords}（最近{days}天内）")
    except Exception as e:
        errors.append(f"查重检查失败: {str(e)}")

    passed = len(errors) == 0

    return {
        'passed': passed,
        'errors': errors,
        'warnings': []
    }


def check_all_gates(html_content: str, content_data: Dict[str, str],
                    entity: str = None, action: str = None, keywords: str = None,
                    db_path: str = None, days: int = 5) -> Dict[str, Any]:
    """
    执行所有Gate检查

    Args:
        html_content: HTML内容
        content_data: 内容数据字典
        entity: 主体（用于历史查重）
        action: 动作（用于历史查重）
        keywords: 关键词（用于历史查重）
        db_path: 数据库路径
        days: 查重天数窗口

    Returns:
        综合检查结果字典
    """
    results = {
        'structure': check_structure(html_content, content_data),
        'facts': check_facts(content_data),
        'html_compliance': check_html_compliance(html_content),
        'content_duplicate': check_duplicate_content(html_content),
    }

    # 如果提供了查重参数，执行历史查重
    if entity and action and keywords:
        results['history_duplicate'] = check_history_duplicate(
            entity, action, keywords, db_path, days
        )

    # 汇总结果
    all_passed = all(result['passed'] for result in results.values())
    all_errors = []
    all_warnings = []

    for gate_name, result in results.items():
        all_errors.extend([f"[{gate_name}] {err}" for err in result['errors']])
        all_warnings.extend([f"[{gate_name}] {warn}" for warn in result['warnings']])

    return {
        'all_passed': all_passed,
        'gates': results,
        'failures': all_errors,
        'warnings': all_warnings
    }


def print_gate_result(result: Dict[str, Any], verbose: bool = True):
    """
    打印Gate检查结果

    Args:
        result: check_all_gates的返回结果
        verbose: 是否输出详细信息
    """
    print("=" * 50)
    print("交付Gate检查结果")
    print("=" * 50)

    if result['all_passed']:
        print("✓ 所有检查通过")
    else:
        print("✗ 检查失败")

    print()

    if verbose:
        # 打印错误
        if result['failures']:
            print("错误列表:")
            for error in result['failures']:
                print(f"  ✗ {error}")
            print()

        # 打印警告
        if result['warnings']:
            print("警告列表:")
            for warning in result['warnings']:
                print(f"  ⚠ {warning}")
            print()

        # 打印各Gate详情
        print("各Gate详情:")
        for gate_name, gate_result in result['gates'].items():
            status = "✓ 通过" if gate_result['passed'] else "✗ 失败"
            print(f"  {gate_name}: {status}")

            if gate_result['errors']:
                for err in gate_result['errors']:
                    print(f"    - {err}")

            if gate_result['warnings']:
                for warn in gate_result['warnings']:
                    print(f"    ! {warn}")

    print("=" * 50)


if __name__ == "__main__":
    # 测试用例
    test_html = '''
    <!DOCTYPE html>
    <html>
    <head><meta charset="utf-8"></head>
    <body>
        <div class="headline">
            <h1>测试标题</h1>
            <p>测试内容</p>
            <img src="https://example.com/image.jpg" alt="测试图片">
        </div>
        <div class="sub-news">行业要闻</div>
        <div class="references">参考来源</div>
    </body>
    </html>
    '''

    test_content = {
        'headline_title': '测试标题',
        'headline_content': '<p>这是测试内容，用于验证Gate检查功能。这个内容应该足够长。</p>',
        'sub_news': '<div>行业要闻内容</div>',
        'publish_date': '2026-01-25'
    }

    print("执行所有Gate检查...")
    result = check_all_gates(
        html_content=test_html,
        content_data=test_content,
        entity='Test',
        action='测试',
        keywords='Gate,测试'
    )

    print()
    print_gate_result(result, verbose=True)
