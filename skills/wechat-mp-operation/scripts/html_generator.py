#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
HTML排版生成模块
功能：基于模板生成符合微信公众号排版的HTML
"""

import os
from typing import Dict, Any
from datetime import datetime


def generate_html(template_path: str, content_data: Dict[str, str]) -> str:
    """
    基于模板生成HTML内容

    Args:
        template_path: HTML模板文件路径
        content_data: 内容数据字典，包含以下字段：
            - headline_title: 头条标题
            - headline_content: 头条内容HTML
            - sub_news: 行业要闻HTML
            - data_charts: 数据与榜单HTML
            - tools: 宝藏工具HTML
            - daily_quote: 每日一言HTML
            - references: 参考来源HTML
            - issue_number: 期数
            - publish_date: 发布日期

    Returns:
        生成的HTML字符串
    """
    # 读取模板
    with open(template_path, 'r', encoding='utf-8') as f:
        template = f.read()

    # 填充内容
    html = template.replace('{{headline_title}}', content_data.get('headline_title', ''))
    html = html.replace('{{headline_content}}', content_data.get('headline_content', ''))
    html = html.replace('{{sub_news}}', content_data.get('sub_news', ''))
    html = html.replace('{{data_charts}}', content_data.get('data_charts', ''))
    html = html.replace('{{tools}}', content_data.get('tools', ''))
    html = html.replace('{{daily_quote}}', content_data.get('daily_quote', ''))
    html = html.replace('{{references}}', content_data.get('references', ''))
    html = html.replace('{{issue_number}}', content_data.get('issue_number', '1'))
    html = html.replace('{{publish_date}}', content_data.get('publish_date', datetime.now().strftime('%Y-%m-%d')))

    # 填充头图信息
    html = html.replace('{{header_cover_path}}', content_data.get('header_cover_path', ''))
    html = html.replace('{{header_cover_alt}}', content_data.get('header_cover_alt', '头图'))
    html = html.replace('{{header_caption}}', content_data.get('header_caption', ''))

    # 填充动态主题标签
    theme_tags = content_data.get('theme_tags', [])
    if isinstance(theme_tags, str):
        # 如果是字符串，直接使用
        theme_tags_html = theme_tags
    elif isinstance(theme_tags, list):
        # 如果是列表，生成标签HTML
        theme_tags_html = '\n'.join([
            f'<span style="padding:6px 16px;background-color:#f0f0f0;border-radius:20px;font-size:14px;color:#666666;">{tag}</span>'
            for tag in theme_tags
        ])
    else:
        theme_tags_html = ''

    html = html.replace('{{theme_tags}}', theme_tags_html)

    return html


def validate_html(html_content: str) -> Dict[str, Any]:
    """
    验证HTML内容的完整性

    Args:
        html_content: HTML内容

    Returns:
        验证结果字典，包含:
            - is_valid: 是否有效
            - errors: 错误列表
            - warnings: 警告列表
    """
    errors = []
    warnings = []

    # 检查必需的标签
    required_tags = ['<!DOCTYPE html>', '<html', '<head>', '<body>']
    for tag in required_tags:
        if tag not in html_content:
            errors.append(f"缺少必需标签: {tag}")

    # 检查必需的占位符是否被替换
    placeholders = [
        '{{headline_title}}',
        '{{headline_content}}',
        '{{publish_date}}'
    ]
    for placeholder in placeholders:
        if placeholder in html_content:
            errors.append(f"占位符未替换: {placeholder}")

    # 检查常见的HTML问题
    if html_content.count('<body') != 1:
        errors.append("body标签数量不正确")

    if '<style' not in html_content:
        warnings.append("缺少style标签，可能导致排版异常")

    # 检查图片标签
    if '<img' in html_content and 'src=' not in html_content:
        errors.append("存在img标签但缺少src属性")

    is_valid = len(errors) == 0

    return {
        'is_valid': is_valid,
        'errors': errors,
        'warnings': warnings
    }


def save_html(html_content: str, output_path: str):
    """
    保存HTML内容到文件

    Args:
        html_content: HTML内容
        output_path: 输出文件路径
    """
    # 确保目录存在
    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(html_content)


def generate_module_html(module_type: str, items: list) -> str:
    """
    生成特定模块的HTML

    Args:
        module_type: 模块类型（'sub_news', 'data_charts', 'tools', 'references'）
        items: 项目列表，每个项目为字典

    Returns:
        模块HTML字符串
    """
    html_parts = []

    if module_type == 'sub_news':
        # 行业要闻
        for idx, item in enumerate(items[:5], 1):
            html_parts.append(f'''
            <div class="sub-news-item">
                <span class="item-num">{idx}.</span>
                <span class="item-title">{item.get('title', '')}</span>
                <span class="item-summary">{item.get('summary', '')}</span>
            </div>
            ''')

    elif module_type == 'data_charts':
        # 数据与榜单
        for item in items[:3]:
            html_parts.append(f'''
            <div class="data-item">
                <div class="data-title">{item.get('title', '')}</div>
                <div class="data-content">{item.get('content', '')}</div>
            </div>
            ''')

    elif module_type == 'tools':
        # 宝藏工具
        for item in items[:2]:
            html_parts.append(f'''
            <div class="tool-item">
                <div class="tool-name">{item.get('name', '')}</div>
                <div class="tool-desc">{item.get('description', '')}</div>
                <div class="tool-link">{item.get('link', '')}</div>
            </div>
            ''')

    elif module_type == 'references':
        # 参考来源
        for item in items:
            html_parts.append(f'''
            <div class="reference-item">
                <span class="ref-source">{item.get('source', '')}</span>
                <span class="ref-title">{item.get('title', '')}</span>
            </div>
            ''')

    return ''.join(html_parts)


if __name__ == "__main__":
    # 测试生成HTML（需要模板文件）
    test_content = {
        "headline_title": "测试标题",
        "headline_content": "<p>测试内容</p>",
        "sub_news": "<div>行业要闻</div>",
        "data_charts": "<div>数据榜单</div>",
        "tools": "<div>宝藏工具</div>",
        "daily_quote": "<div>每日一言</div>",
        "references": "<div>参考来源</div>",
        "issue_number": "1",
        "publish_date": "2026-01-25"
    }

    # 检查模板是否存在
    template_path = "../references/html_template_v2.1.html"
    if os.path.exists(template_path):
        html = generate_html(template_path, test_content)
        print("HTML生成成功")
        print(f"长度: {len(html)} 字符")

        # 验证HTML
        result = validate_html(html)
        print(f"\n验证结果: {result['is_valid']}")
        if result['errors']:
            print("错误:", result['errors'])
        if result['warnings']:
            print("警告:", result['warnings'])
    else:
        print(f"模板文件不存在: {template_path}")
