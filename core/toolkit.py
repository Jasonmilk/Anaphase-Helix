import os, subprocess, requests

class SurgeonToolkit:
    def __init__(self, workspace):
        self.workspace = workspace
        self.api_key = os.getenv("TAVILY_API_KEY")

    def tavily_search(self, query):
        """连接物理互联网"""
        if not self.api_key: return "[❌] 未配置 TAVILY_API_KEY"
        payload = {
            "api_key": self.api_key,
            "query": query,
            "search_depth": "basic",
            "max_results": 3
        }
        try:
            response = requests.post("https://api.tavily.com/search", json=payload, timeout=15)
            data = response.json()
            results = [f"标题: {r['title']}\n内容: {r['content']}\n链接: {r['url']}" for r in data.get('results', [])]
            return "\n\n".join(results) if results else "未找到相关信息。"
        except Exception as e:
            return f"[搜索失败] {str(e)}"

    def get_system_map(self, depth=3):
        """建立环境认知"""
        try:
            return subprocess.getoutput(f"tree -L {depth} {self.workspace}")
        except:
            return "tree 命令执行失败。"

    def list_dir(self, path="."):
        full_path = os.path.join(self.workspace, path)
        try:
            return "\n".join(os.listdir(full_path))
        except:
            return f"无法访问路径: {path}"
