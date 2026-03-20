import os
import importlib.util
import sys
import traceback

class SkillRegistry:
    def __init__(self, workspace):
        self.workspace = workspace
        self.skills = {}
        self.skills_dir = "/opt/anaphase/core/skills"
        os.makedirs(self.skills_dir, exist_ok=True)
        self.load_all()

    def load_all(self):
        """动态扫描并加载技能，具备物理容错能力"""
        self.skills = {}
        for filename in os.listdir(self.skills_dir):
            if filename.endswith(".py") and not filename.startswith("__"):
                name = filename[:-3]
                filepath = os.path.join(self.skills_dir, filename)
                
                try:
                    # 尝试加载模块
                    spec = importlib.util.spec_from_file_location(name, filepath)
                    mod = importlib.util.module_from_spec(spec)
                    if name in sys.modules:
                        del sys.modules[name]
                    spec.loader.exec_module(mod)
                    
                    if hasattr(mod, "register"):
                        mod.register(self)
                except Exception:
                    # 【核心优化】：捕获语法错误或其他异常，跳过损坏的技能包，不中断主进程
                    print(f"\n[Registry Warning] 技能文件 {filename} 存在语法错误或物理损坏，已跳过。")
                    continue
        return len(self.skills)

    def add(self, name, func, desc):
        self.skills[name] = {"func": func, "desc": desc}

    def execute(self, name, args_raw):
        # 处理带 toolkit. 前缀的情况
        clean_name = name.replace('toolkit.', '')
        if clean_name not in self.skills:
            self.load_all() # 尝试热重载
            if clean_name not in self.skills:
                return f"[Error] Skill '{clean_name}' not found."
        try:
            return self.skills[clean_name]["func"](self.workspace, args_raw)
        except Exception as e:
            return f"[Skill Exception] {str(e)}"

    def get_docs(self):
        docs = []
        for k, v in self.skills.items():
            # 简化输出格式，压缩 Token 占用
            docs.append(f"- toolkit.{k}(args): {v['desc'].split('参数:')[0]}")
        return "\n".join(docs)
