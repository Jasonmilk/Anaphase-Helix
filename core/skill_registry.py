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
        """扫描并加载/重载所有技能"""
        self.skills = {}
        for filename in os.listdir(self.skills_dir):
            if filename.endswith(".py") and not filename.startswith("__"):
                name = filename[:-3]
                filepath = os.path.join(self.skills_dir, filename)
                
                # 强行重载模块，确保新写入的代码生效
                spec = importlib.util.spec_from_file_location(name, filepath)
                mod = importlib.util.module_from_spec(spec)
                if name in sys.modules:
                    del sys.modules[name]
                spec.loader.exec_module(mod)
                
                if hasattr(mod, "register"):
                    mod.register(self)
        return len(self.skills)

    def add(self, name, func, desc):
        self.skills[name] = {"func": func, "desc": desc}

    def execute(self, name, args_raw):
        if name not in self.skills:
            # 尝试最后一次热加载，万一是刚生成的技能
            self.load_all()
            if name not in self.skills:
                return f"[错误] 未识别技能: {name}"
        try:
            return self.skills[name]["func"](self.workspace, args_raw)
        except Exception as e:
            return f"[异常] {str(e)}"

    def get_docs(self):
        return "\n".join([f"- `toolkit.{k}(args)`: {v['desc']}" for k, v in self.skills.items()])
