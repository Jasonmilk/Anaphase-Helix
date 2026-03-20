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
        if name not in self.skills: self.load_all()
        # 即使模型带了 toolkit. 前缀，执行前也要剥离
        clean_name = name.replace('toolkit.', '')
        if clean_name not in self.skills: 
            return f"Error: Skill '{clean_name}' unknown."
        return self.skills[clean_name]["func"](self.workspace, args_raw)

    def get_docs(self):
        """物理强化：直接在文档中展示 toolkit. 前缀，诱导模型正确输出"""
        docs = []
        for k, v in self.skills.items():
            desc_parts = v['desc'].split('参数:')
            usage = desc_parts[1].strip() if len(desc_parts) > 1 else "..."
            # 这里是关键：直接展示 toolkit.
            docs.append(f"- toolkit.{k}({usage})")
        return "\n".join(docs)
