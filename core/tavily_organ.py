import os
import requests
from dotenv import load_dotenv

load_dotenv("/opt/anaphase/.env")

def search_web(query: str):
    """
    访问互联网获取实时知识，为你的逻辑提供真实世界锚点。
    """
    api_key = os.getenv("TAVILY_API_KEY")
    if not api_key:
        return "Error: TAVILY_API_KEY missing."
    
    url = "https://api.tavily.com/search"
    payload = {
        "api_key": api_key,
        "query": query,
        "search_depth": "advanced", # 深度搜索，获取更严谨的知识
        "max_results": 5 
    }
    
    try:
        response = requests.post(url, json=payload, timeout=30)
        response.raise_for_status()
        data = response.json()
        results = [f"[{i+1}] Source: {r['url']}\nContent: {r['content']}" for i, r in enumerate(data.get('results', []))]
        return "\n\n---\n\n".join(results) if results else "No knowledge found for this query."
    except Exception as e:
        return f"Access Denied: {str(e)}"

if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        print(search_web(sys.argv[1]))
