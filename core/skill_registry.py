import os
import importlib.util
import sys

class SkillRegistry:
    def __init__(self, workspace):
        self.workspace = workspace
        self.skills = {}
        self.skills_dir = "/opt/anaphase/core/skills"
        os.makedirs(self.skills_dir, exist_ok=True)
        self.load_all()

    def load_all(self):
        """免疫系统：跳过所有物理损坏的技能文件"""
        self.skills = {}
        for filename in os.listdir(self.skills_dir):
            if filename.endswith(".py") and not filename.startswith("__"):
                name = filename[:-3]
                filepath = os.path.join(self.skills_dir, filename)
                
                try:
                    spec = importlib.util.spec_from_file_location(name, filepath)
                    mod = importlib.util.module_from_spec(spec)
                    if name in sys.modules: del sys.modules[name]
                    spec.loader.exec_module(mod)
                    if hasattr(mod, "register"):
                        mod.register(self)
                except Exception as e:
                    # 【核心加固】：不抛出异常，只记录警告
                    print(f"⚠️ [免疫系统] 跳过损坏技能 {filename}: {str(e)}")
                    continue
        return len(self.skills)

    def add(self, name, func, desc):
        self.skills[name] = {"func": func, "desc": desc}

    def execute(self, name, args_raw):
        clean_name = name.replace('toolkit.', '')
        if clean_name not in self.skills:
            self.load_all() # 热尝试
            if clean_name not in self.skills:
                return f"[错误] 技能 {clean_name} 尚未挂载或物理损坏。"
        try:
            return self.skills[clean_name]["func"](self.workspace, args_raw)
        except Exception as e:
            return f"[执行异常] {str(e)}"

    def get_docs(self):
        docs = []
        for k, v in self.skills.items():
            docs.append(f"- toolkit.{k}(args): {v['desc'].split('参数:')[0]}")
        return "\n".join(docs)
