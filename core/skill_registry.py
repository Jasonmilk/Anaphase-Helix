import os
import importlib.util

class SkillRegistry:
    def __init__(self, workspace):
        self.workspace = workspace
        self.skills = {}
        self.skills_dir = "/opt/anaphase/core/skills"
        os.makedirs(self.skills_dir, exist_ok=True)
        self._load_skills()

    def _load_skills(self):
        """动态扫描并加载所有 .py 技能包"""
        for filename in os.listdir(self.skills_dir):
            if filename.endswith(".py") and not filename.startswith("__"):
                name = filename[:-3]
                filepath = os.path.join(self.skills_dir, filename)
                spec = importlib.util.spec_from_file_location(name, filepath)
                mod = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(mod)
                # 寻找并执行注册函数
                if hasattr(mod, "register"):
                    mod.register(self)

    def add(self, name, func, desc):
        """供子技能调用的注册接口"""
        self.skills[name] = {"func": func, "desc": desc}

    def execute(self, name, args_raw):
        """执行被调用的技能"""
        if name not in self.skills:
            return f"[错误] 未知技能或尚未安装: {name}。请检查拼写。"
        try:
            return self.skills[name]["func"](self.workspace, args_raw)
        except Exception as e:
            return f"[技能执行异常] {str(e)}"

    def get_docs(self):
        """自动生成技能说明书，喂给 LLM"""
        return "\n".join([f"- `toolkit.{k}({v['desc'].split('参数:')[1].strip() if '参数:' in v['desc'] else 'args'})`: {v['desc'].split('参数:')[0].strip()}" for k, v in self.skills.items()])
