#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
图片URL修复模块
功能：将短链接替换为直接可访问的图床链接
"""

import re
from typing import Tuple, Dict
import sys

# 动态导入requests，支持coze_workload_identity
try:
    from coze_workload_identity import requests
except ImportError:
    try:
        import requests
    except ImportError:
        print("错误: 需要安装 requests 库")
        sys.exit(1)


def resolve_short_url(short_url: str, max_redirects: int = 5,
                      timeout: int = 10) -> Tuple[str, bool, str]:
    """
    解析短链接，获取最终的真实URL

    Args:
        short_url: 短链接URL
        max_redirects: 最大重定向次数
        timeout: 请求超时时间（秒）

    Returns:
        (真实URL, 是否成功, 错误信息)
    """
    try:
        # 使用allow_redirects参数自动处理重定向
        response = requests.head(
            short_url,
            allow_redirects=True,
            timeout=timeout,
            verify=False
        )

        if response.status_code == 200:
            return (response.url, True, "")
        else:
            return (short_url, False, f"HTTP状态码: {response.status_code}")

    except requests.exceptions.Timeout:
        return (short_url, False, "请求超时")
    except requests.exceptions.TooManyRedirects:
        return (short_url, False, "重定向次数超过限制")
    except requests.exceptions.RequestException as e:
        return (short_url, False, f"请求异常: {str(e)}")
    except Exception as e:
        return (short_url, False, f"未知错误: {str(e)}")


def is_short_url(url: str) -> bool:
    """
    判断是否为短链接

    Args:
        url: URL字符串

    Returns:
        True表示是短链接，False表示不是
    """
    if not url or not isinstance(url, str):
        return False

    # 常见短链接域名
    short_domains = [
        'bit.ly', 't.co', 'goo.gl', 'tinyurl.com', 'ow.ly',
        'is.gd', 'buff.ly', 'rebrand.ly', 'short.io', 'bit.do'
    ]

    url_lower = url.lower()
    for domain in short_domains:
        if domain in url_lower:
            return True

    # 检查URL结构（不含路径或路径很短）
    parsed = re.match(r'^https?://[^/]+$', url)
    if parsed:
        return True

    return False


def fix_image_urls(html_content: str, max_redirects: int = 5,
                   timeout: int = 10, verbose: bool = False) -> Tuple[str, Dict[str, any]]:
    """
    批量修复HTML中的图片URL

    Args:
        html_content: HTML内容字符串
        max_redirects: 最大重定向次数
        timeout: 请求超时时间（秒）
        verbose: 是否输出详细日志

    Returns:
        (修复后的HTML, 统计信息)
        统计信息包含:
            - total: 总图片数
            - fixed: 成功修复数
            - failed: 修复失败数
            - skipped: 跳过数（非短链接）
    """
    # 统计信息
    stats = {
        'total': 0,
        'fixed': 0,
        'failed': 0,
        'skipped': 0
    }

    # 匹配img标签的src属性
    img_pattern = re.compile(r'<img[^>]+src=["\']([^"\']+)["\'][^>]*>', re.IGNORECASE)

    def replace_img_tag(match):
        """替换img标签中的URL"""
        nonlocal stats
        stats['total'] += 1

        original_tag = match.group(0)
        original_url = match.group(1)

        # 检查是否为短链接
        if not is_short_url(original_url):
            stats['skipped'] += 1
            if verbose:
                print(f"跳过（非短链接）: {original_url}")
            return original_tag

        # 解析短链接
        real_url, success, error = resolve_short_url(original_url, max_redirects, timeout)

        if success:
            stats['fixed'] += 1
            if verbose:
                print(f"✓ 修复: {original_url} -> {real_url}")
            # 替换src属性
            return original_tag.replace(f'src="{original_url}"', f'src="{real_url}"').replace(
                f"src='{original_url}'", f"src='{real_url}'"
            )
        else:
            stats['failed'] += 1
            if verbose:
                print(f"✗ 失败: {original_url} ({error})")
            return original_tag

    # 执行替换
    fixed_html = img_pattern.sub(replace_img_tag, html_content)

    return fixed_html, stats


def validate_image_urls(html_content: str) -> Dict[str, any]:
    """
    验证HTML中的图片URL是否有效

    Args:
        html_content: HTML内容字符串

    Returns:
        验证结果字典，包含:
            - total: 总图片数
            - valid: 有效URL数
            - invalid: 无效URL数
            - short_links: 短链接数
    """
    result = {
        'total': 0,
        'valid': 0,
        'invalid': 0,
        'short_links': 0
    }

    img_pattern = re.compile(r'<img[^>]+src=["\']([^"\']+)["\'][^>]*>', re.IGNORECASE)
    matches = img_pattern.findall(html_content)

    result['total'] = len(matches)

    for url in matches:
        # 检查是否为短链接
        if is_short_url(url):
            result['short_links'] += 1
            result['invalid'] += 1
        else:
            # 简单验证URL格式
            if url.startswith('http://') or url.startswith('https://'):
                result['valid'] += 1
            else:
                result['invalid'] += 1

    return result


if __name__ == "__main__":
    # 测试用例
    test_html = '''
    <div>
        <img src="https://bit.ly/test1" alt="测试图片1">
        <img src="https://example.com/image.jpg" alt="测试图片2">
        <img src="https://t.co/test3" alt="测试图片3">
    </div>
    '''

    print("原始HTML:")
    print(test_html)
    print()

    print("验证图片URL:")
    validation = validate_image_urls(test_html)
    print(f"总图片数: {validation['total']}")
    print(f"有效URL: {validation['valid']}")
    print(f"无效URL: {validation['invalid']}")
    print(f"短链接数: {validation['short_links']}")
    print()

    print("修复图片URL:")
    fixed_html, stats = fix_image_urls(test_html, verbose=True)
    print()
    print(f"统计信息:")
    print(f"  总数: {stats['total']}")
    print(f"  修复成功: {stats['fixed']}")
    print(f"  修复失败: {stats['failed']}")
    print(f"  跳过: {stats['skipped']}")
    print()
    print("修复后的HTML:")
    print(fixed_html)
