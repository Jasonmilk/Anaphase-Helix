# -*- coding: utf-8 -*-
import json

def process_json_file(file_path):
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        return data
    except json.JSONDecodeError:
        print("JSON 解析失败")
        return None
    except FileNotFoundError:
        print("文件未找到")
        return None
    except Exception as e:
        print(f"未知错误: {e}")
        return None

if __name__ == '__main__':
    target = '/opt/anaphase/workspace/data.json'
    result = process_json_file(target)
    if result:
        print(f"成功加载数据: {result}")
