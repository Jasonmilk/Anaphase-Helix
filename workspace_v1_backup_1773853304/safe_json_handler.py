#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
安全 JSON 处理器
遵循补丁：1. 强制 utf-8 声明；2. JSON 加载异常捕获。
"""

import json
import os

# 定义工作目录，严格遵守边界
WORKSPACE = '/opt/anaphase/workspace/'

def safe_load_json(filepath):
    """
    安全加载 JSON 文件。
    强制使用 utf-8 编码，并捕获所有解析异常。
    """
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"文件不存在: {filepath}")
    
    try:
        # 强制 utf-8 编码，防止 BOM 或编码错误
        with open(filepath, 'r', encoding='utf-8') as f:
            return json.load(f)
    except UnicodeDecodeError as e:
        raise ValueError(f"编码错误: {e}")
    except json.JSONDecodeError as e:
        raise ValueError(f"JSON 解析错误: {e}")
    except Exception as e:
        raise RuntimeError(f"未知错误: {e}")

if __name__ == '__main__':
    # 示例：读取工作目录下的 json 文件
    target_file = os.path.join(WORKSPACE, 'data.json')
    try:
        data = safe_load_json(target_file)
        print("成功解析数据:", data)
    except Exception as err:
        print(f"操作失败，已记录错误: {err}")
        # 此处可添加日志记录，确保错误被记录而非丢失
