      
"""
法条检索脚本 - 调用Workflow API进行法条检索
"""
import argparse
import json
import os
import sys
import time
import requests
from pathlib import Path
from typing import Dict, Any

# Configuration
API_URL = "https://sd500jqlh74m0t5ldnsig.apigateway-cn-beijing-inner.volceapi.com/workflow/legal_new_skills"
WORKFLOW_ID = "7604842799560687625"
BEARER_TOKEN = os.getenv('identity_ticket')
TIMEOUT_SECONDS = 600  # 10 minutes

BLACK_ITEM_SET = {
}

class LawStore:
    _gid_set = set() # 全局去重
    _law_list = [] # 案例列表

    def __init__(self):
        pass
    
    def add_law(self, law):
        # 去重
        if law['Gid'] in self._gid_set:
            return
        self._gid_set.add(law['Gid'])
        self._law_list.append(law)

    def get_laws(self, n=10):
        return self._law_list[:n]

    def print_laws(self):
        with open("b.json", 'w') as f:
            json.dump(self._law_list, f, ensure_ascii=False, indent=2)

def log(data):
    pass

def print_debug(msg):
    """打印调试信息"""
    print(msg, flush=True)
    try:
        with open("search_log.txt", "a") as f:
            f.write(f"{time.strftime('%H:%M:%S')} - {msg}\n")
    except:
        pass

def parse_law_input(input_path: str):
    """
    解析JSON格式的法条检索请求
    
    输入格式示例：
    {
      "parameters": {
        "title": "检索描述...",
        "fulltext": "检索描述..."
      }
    }
    Returns: Workflow API所需的payload
    """
    try:
        with open(input_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        result_groups = []
        for item in data:
            if 'parameters' not in item:
                raise ValueError("缺少必需字段: parameters")
            if ('title' not in item['parameters']) and ('fulltext' not in item['parameters']):
                raise ValueError("缺少必需字段: parameters.title 和 parameters.fulltext 参数最少需要有一个")

            result_groups.append({
                'workflow_id': WORKFLOW_ID,
                'parameters': item['parameters']
            })            
        return result_groups
        
    except json.JSONDecodeError as e:
        raise ValueError(f"JSON格式错误: {e}")
    except FileNotFoundError:
        raise ValueError(f"文件未找到: {input_path}")

def extract_items(origin_json):
    output = json.loads(origin_json['data']).get('output', [])
    # remove invalid attributes
    for item in output:
        tobe_removed = []
        for k in item.keys():
            if (k in BLACK_ITEM_SET) or (item[k] is None):
                tobe_removed.append(k)
                continue
            vt = type(item[k])
            vv = item[k]
            if (vt == str and vv == '') or (vt == list and len(vv) == 0) or (vt == dict and len(vv) == 0):
                tobe_removed.append(k)
        for k in tobe_removed:
            item.pop(k, None)
    return output

def call_workflow_api(payload: Dict[str, Any]) -> Dict:
    """
    调用Workflow API进行法条检索
    
    Args:
        payload: Workflow API请求payload
        output_path: 输出JSON文件路径
        
    Returns: API响应数据
    """
    
    try:
        headers = {
          "Content-Type": "application/json",
          "Authorization": f"Bearer {BEARER_TOKEN}"          
        }
        response = requests.post(
            API_URL,
            headers = headers,
            json=payload,
            timeout=TIMEOUT_SECONDS
        )
        response.raise_for_status()
        log(response.text)

        result = extract_items(response.json())
        return result
        
    except requests.exceptions.Timeout:
        raise Exception(f"请求超时（{TIMEOUT_SECONDS}秒），请稍后重试")
    except requests.exceptions.RequestException as e:
        raise Exception(f"API调用失败: {str(e)}")

def search_laws(input_file: str, output_file: str):
    """
    执行法条检索主流程
    
    Args:
        input_file: 输入JSON文件路径
        output_file: 输出JSON文件路径
    """
    try:
        print_debug("🚀 开始法条检索...")
        
        payload_groups = parse_law_input(input_file)
        print_debug(f"📖 解析检索请求成功，共{len(payload_groups)}组检索词。开始调用 API...")
        
        law_store = LawStore()
        for payload in payload_groups:
            result = call_workflow_api(payload)
            print_debug(f"✅ API调用成功，返回{len(result)}条结果")
            for item in result:
                law_store.add_law(item)
        
        result_groups = law_store.get_laws()
        print_debug(f"✅ 所有 API 均调用完成，去重排序后共获取{len(result_groups)}条结果")

        data = json.dumps(result_groups, ensure_ascii=False, indent=2)

        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(data)

        print_debug(f"📄 检索结果已保存到: {output_file}")
        print(f"检索结果全文如下：\n{data}")

    except Exception as e:
        print_debug(f"❌ 检索失败: {str(e)}")
        sys.exit(1)

if __name__ == "__main__":
    print_debug("🚀 law_search.py 启动...")
    
    parser = argparse.ArgumentParser(
        description="法条检索 - 调用Workflow API进行法条检索",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    
    parser.add_argument(
        "--input",
        required=True,
        help="输入JSON文件路径，格式参考 references/law-data-format.md"
    )
    parser.add_argument(
        "--output",
        required=True,
        help="输出JSON文件路径，保存检索结果"
    )
    
    args = parser.parse_args()
    
    search_laws(args.input, args.output)

    