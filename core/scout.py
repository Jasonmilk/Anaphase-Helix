import os
import json
import requests
from datetime import datetime, timedelta
from core.config import settings

class GitHubScout:
    """合规索敌雷达：严格锁定 Helix 的能力边界，只拉取新手级真实 Bug"""
    
    def __init__(self):
        self.api_url = "https://api.github.com/search/issues"
        self.headers = {
            "Accept": "application/vnd.github.v3+json",
            "Authorization": f"token {settings.GITHUB_TOKEN}" if settings.GITHUB_TOKEN else ""
        }
        self.pool_file = os.path.join(settings.WORKSPACES_DIR, "Target_Issue_Pool.json")
        # 红线 4：合规性前置过滤 (只接受宽松开源协议)
        self.allowed_licenses =['mit', 'apache-2.0', 'bsd-2-clause', 'bsd-3-clause']

    def _check_repo_license(self, repo_url):
        """检查仓库的开源协议是否合规"""
        try:
            res = requests.get(repo_url, headers=self.headers, timeout=10)
            if res.status_code == 200:
                repo_data = res.json()
                license_info = repo_data.get("license")
                if license_info and license_info.get("key") in self.allowed_licenses:
                    return True
        except Exception:
            pass
        return False

    def fetch_targets(self, limit=5):
        """拉取符合能力边界的 Issue"""
        print("\n[Scout 雷达] 正在扫描 GitHub 开源生态...")
        
        # 边界锁定：Python语言 + 开放状态 + Bug + 新手友好 + 非归档项目
        # 增加 updated 过滤，只找最近 30 天内活跃的项目
        thirty_days_ago = (datetime.utcnow() - timedelta(days=30)).strftime('%Y-%m-%d')
        query = f'is:open is:issue label:"good first issue" label:bug language:python updated:>={thirty_days_ago} archived:false'
        
        params = {
            "q": query,
            "sort": "updated",
            "order": "desc",
            "per_page": 30 # 多拉取一些，因为要过滤 License
        }

        try:
            res = requests.get(self.api_url, headers=self.headers, params=params, timeout=15)
            if res.status_code != 200:
                print(f"[Scout Error] API 拒绝访问: {res.status_code} - {res.text}")
                return False

            items = res.json().get("items", [])
            valid_targets =[]

            for item in items:
                if len(valid_targets) >= limit:
                    break
                
                # 过滤掉评论太多（可能已经被别人抢了或太复杂）的 Issue
                if item.get("comments", 0) > 5:
                    continue
                
                # 过滤掉内容太长（超过 Helix Token 承载极限）的 Issue
                body = item.get("body", "") or ""
                if len(body) > 3000:
                    continue

                repo_url = item.get("repository_url")
                print(f"  -> 发现潜在目标: {item['html_url']} | 校验合规性...")
                
                if self._check_repo_license(repo_url):
                    valid_targets.append({
                        "issue_number": item["number"],
                        "title": item["title"],
                        "url": item["html_url"],
                        "repo_url": repo_url,
                        "body": body[:1500] + "...(截断)" if len(body) > 1500 else body,
                        "status": "PENDING"
                    })
                    print("     [通过] 协议合规，已锁定。")
                else:
                    print("     [拦截] 协议不符或无 License，已丢弃。")

            if valid_targets:
                os.makedirs(os.path.dirname(self.pool_file), exist_ok=True)
                with open(self.pool_file, "w", encoding="utf-8") as f:
                    json.dump(valid_targets, f, ensure_ascii=False, indent=2)
                print(f"\n[Scout 完毕] 成功捕获 {len(valid_targets)} 个符合 Helix 能力边界的真实 Bug，已存入任务池。")
                return True
            else:
                print("\n[Scout 完毕] 本轮未发现完美匹配的目标。")
                return False

        except Exception as e:
            print(f"[Scout Error] 雷达扫描崩溃: {e}")
            return False

if __name__ == "__main__":
    scout = GitHubScout()
    scout.fetch_targets(limit=3)
