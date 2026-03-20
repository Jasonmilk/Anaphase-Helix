import os, importlib.util, sys

class SkillRegistry:
    def __init__(self, workspace):
        self.workspace = workspace
        self.skills = {}
        self.skills_dir = "/opt/anaphase/core/skills"
        os.makedirs(self.skills_dir, exist_ok=True)
        self.load_all()

    def load_all(self):
        self.skills = {}
        for filename in os.listdir(self.skills_dir):
            if filename.endswith(".py") and not filename.startswith("__"):
                name = filename[:-3]
                spec = importlib.util.spec_from_file_location(name, os.path.join(self.skills_dir, filename))
                mod = importlib.util.module_from_spec(spec)
                if name in sys.modules: del sys.modules[name]
                spec.loader.exec_module(mod)
                if hasattr(mod, "register"): mod.register(self)

    def add(self, name, func, desc):
        self.skills[name] = {"func": func, "desc": desc}

    def execute(self, name, args_raw):
        clean_name = name.replace('toolkit.', '')
        if clean_name not in self.skills: return f"Error: Skill '{clean_name}' unknown."
        return self.skills[clean_name]["func"](self.workspace, args_raw)

    def get_docs(self):
        """精准诱导：去掉(args)占位符，直接给范例，防止 4B 产生 args: 这种幻觉"""
        docs = []
        for k, v in self.skills.items():
            # 简化输出，只给名称和功能描述
            desc = v['desc'].split('参数:')[0].strip()
            docs.append(f"- toolkit.{k}('...') : {desc}")
        return "\n".join(docs)
