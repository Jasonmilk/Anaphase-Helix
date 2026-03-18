# -*- coding: utf-8 -*-
import json

def load_safe_data(file_path):
    """
    安全加载 JSON 数据的函数。
    遵循补丁规则：在使用 json.loads 前必须进行 try-except 异常捕获。
    """
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"错误：文件未找到 - {file_path}")
        return None
    except json.JSONDecodeError:
        print(f"错误：JSON 格式无效 - {file_path}")
        return None
    except Exception as e:
        print(f"发生未知错误：{e}")
        return None

if __name__ == "__main__":
    # 模拟执行
    data = load_safe_data('data/config.json')
    if data:
        print(f"成功加载数据：{data}")
    else:
        print("数据加载失败，跳过后续流程。")
