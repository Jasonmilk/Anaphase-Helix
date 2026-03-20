import requests
import os

def tavily_search(workspace, args_raw):
    query = args_raw.strip().strip("'\"")
    api_key = os.getenv("TAVILY_API_KEY")
    if not api_key: return "[错误] 缺少 TAVILY_API_KEY。"
    
    payload = {"api_key": api_key, "query": query, "search_depth": "basic", "max_results": 2}
    try:
        res = requests.post("https://api.tavily.com/search", json=payload, timeout=15).json()
        results = [f"- {r['title']}: {r['content'][:200]}..." for r in res.get('results',[])]
        return "\n".join(results) if results else "未找到相关信息。"
    except Exception as e:
        return f"[搜索异常] {str(e)}"

def register(registry):
    registry.add("tavily_search", tavily_search, "搜索互联网获取最新知识或报错解决办法。参数: 搜索关键词。")
