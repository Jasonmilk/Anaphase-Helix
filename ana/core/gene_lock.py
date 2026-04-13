import os
import re
import shlex
from typing import Dict, Any

# 安全规则：危险路径模式（Anaphase层约束，非基因锁）
FORBIDDEN_PATH_PATTERNS = [
    r"^/usr(/bin)?(/.*)?$",
    r"^/bin(/.*)?$",
    r"^/sbin(/.*)?$",
    r"^/etc(/.*)?$",
    r"^/boot(/.*)?$",
    r"^/dev(/.*)?$",
    r"^/sys(/.*)?$",
    r"^/proc(/.*)?$",
    r"^~/.ssh(/.*)?$",
    r"^~/.gnupg(/.*)?$",
]

class GeneLockValidator:
    def __init__(self, lock_path: str):
        self.lock_path = lock_path
        self.rules = self._load_rules()

    def _load_rules(self):
        # 简化：默认只检查危险命令
        return {
            "forbidden_commands": ["rm -rf /", "dd if=/dev/zero", "mkfs", ":(){ :|:& };:"],
            "privacy_keywords": ["password", "secret", "token", "api_key"]
        }

    def check(self, tool_name: str, params: Dict[str, Any]) -> bool:
        # 对写操作进行简单校验
        if tool_name in ["ana_kb_write", "ana_cancel"]:
            # 检查参数中是否包含敏感信息
            params_str = str(params).lower()
            for keyword in self.rules["privacy_keywords"]:
                if keyword in params_str:
                    return False
        return True

    def reload(self):
        self.rules = self._load_rules()

    def validate_command_safety(self, command: str, user_confirmed: bool = False):
        """
        校验命令安全性（Anaphase 安全规则，非基因锁）
        返回 (is_safe: bool, reason: str)
        """
        if user_confirmed:
            return True, "user_confirmed"
        
        try:
            tokens = shlex.split(command)
        except ValueError:
            tokens = command.split()
        
        for token in tokens:
            if token.startswith('-'):
                continue
            for pattern in FORBIDDEN_PATH_PATTERNS:
                if re.match(pattern, token):
                    return False, f"路径 {token} 匹配危险模式 {pattern}"
        return True, "safe"
