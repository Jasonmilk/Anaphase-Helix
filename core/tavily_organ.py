import os
import requests
from dotenv import load_dotenv

# 加载环境变量，确保能读取到注入的 TAVILY_API_KEY
load_dotenv("/opt/anaphase/.env")

def search_web(query: str):
    """
    访问互联网获取实时知识。
    警告：此工具受配额限制（全人类/月仅 1000 次），非必要请勿使用！
    隐私红线：严禁在 query 中包含任何关于宿主环境的物理元数据。
    """
    api_key = os.getenv("TAVILY_API_KEY")
    if not api_key:
        return "Error: TAVILY_API_KEY not found."
    
    url = "https://api.tavily.com/search"
    payload = {
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "max_results": 2  # 极致节省：只取前2条，减少对 Token 的压力
    }
    
    try:
        response = requests.post(url, json=payload, timeout=20)
        response.raise_for_status()
        data = response.json()
        results = [f"Source: {r['url']}\nInfo: {r['content']}" for r in data.get('results', [])]
        return "\n---\n".join(results) if results else "No results found."
    except Exception as e:
        return f"Access Failed: {str(e)}"

if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        print(search_web(sys.argv[1]))
