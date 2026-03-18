#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json

def probe():
    try:
        print("Helix is awake.")
        # 模拟一个简单的 JSON 解析尝试，以测试异常捕获
        # data = json.loads('{"status": "ok"}')
    except json.JSONDecodeError:
        print("Caught JSON error gracefully.")
    except Exception as e:
        print(f"Caught generic error: {e}")
    return True

if __name__ == "__main__":
    probe()
