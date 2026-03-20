import os, subprocess

class IsolationWard:
    def __init__(self, issue_id, repo_url):
        self.workspace = f"/opt/anaphase/workspaces/issue_{issue_id}"
        self.repo_url = repo_url

    def clone_repository(self):
        if os.path.exists(self.workspace):
            return True
        print(f"[隔离舱] 物理同步中: {self.repo_url}")
        try:
            subprocess.run(["git", "clone", self.repo_url, self.workspace], check=True)
            return True
        except:
            return False
