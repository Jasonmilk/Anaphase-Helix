import os, importlib.util, sys, re

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
                try:
                    spec = importlib.util.spec_from_file_location(name, os.path.join(self.skills_dir, filename))
                    mod = importlib.util.module_from_spec(spec)
                    if name in sys.modules: del sys.modules[name]
                    spec.loader.exec_module(mod)
                    if hasattr(mod, "register"): mod.register(self)
                except Exception as e:
                    print(f"⚠️ [Registry] 技能损坏 {filename}: {e}")

    def add(self, name, func, desc):
        self.skills[name] = {"func": func, "desc": desc}

    def execute(self, name, args_raw):
        clean_name = name.replace('toolkit.', '')
        if clean_name not in self.skills:
            self.load_all()
            if clean_name not in self.skills: return f"Error: {clean_name} unknown."
        return self.skills[clean_name]["func"](self.workspace, args_raw)

    def get_docs(self, mode="hands"):
        """
        分层智能文档：
        brain: toolkit.name (核心功能)
        hands: toolkit.name(参数名)
        """
        docs = []
        for k, v in self.skills.items():
            desc_text = v['desc']
            # 智能提取：假设格式为 "功能。参数: 路径。"
            main_feat = desc_text.split('。')[0][:10] # 截取前10个字作为功能签
            
            # 提取参数名
            param_match = re.search(r"参数:\s*([^。]+)", desc_text)
            param_name = param_match.group(1).strip() if param_match else "args"
            
            if mode == "brain":
                docs.append(f"- toolkit.{k} ({main_feat})")
            else:
                docs.append(f"- toolkit.{k}('{param_name}')")
        
        return "\n".join(docs)
