"""基因锁校验器 - L0 规则前置拦截"""
from pathlib import Path
from typing import Dict, Any, Optional, List, Tuple
import re


class GeneLockCheckResult:
    """基因锁校验结果"""
    def __init__(self, passed: bool, reason: Optional[str] = None, rule_index: Optional[int] = None):
        self.passed = passed
        self.reason = reason
        self.rule_index = rule_index
    
    def __bool__(self) -> bool:
        return self.passed
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "passed": self.passed,
            "reason": self.reason,
            "rule_index": self.rule_index
        }


class GeneLockValidator:
    """L0 基因锁校验器，负责在工具执行前进行规则拦截。"""
    
    # 高危命令特征（部分匹配即拦截）
    HIGH_RISK_PATTERNS = [
        r"rm\s+(-[rf]+\s+)?/",      # rm -rf /
        r"dd\s+.*of=/dev/",         # dd 写入设备
        r"chmod\s+777",             # chmod 777
        r"mkfs",                     # 格式化文件系统
        r">\s*/etc/",               # 写入/etc
        r"curl.*\|\s*(bash|sh)",    # curl | bash
        r"wget.*\|\s*(bash|sh)",    # wget | bash
        r"sudo\s+rm",               # sudo rm
        r"deltree",                  # Windows 删除树
        r"format\s+c:",             # Windows 格式化
    ]
    
    # 隐私数据关键词
    PRIVACY_KEYWORDS = [
        "password", "passwd", "secret", "token", "api_key", "private_key",
        "credential", "auth_token", "access_token", "refresh_token"
    ]

    def __init__(self, lock_path: Optional[str] = None):
        self.lock_path = Path(lock_path) if lock_path else None
        self._rules: List[str] = []
        self._compiled_patterns: List[Tuple[int, re.Pattern]] = []
        self._load_rules()
        self._compile_patterns()

    def _load_rules(self) -> None:
        """从文件加载 L0 基因锁规则。"""
        if self.lock_path and self.lock_path.exists():
            with open(self.lock_path, "r", encoding="utf-8") as f:
                # 简单解析：每行一条规则，忽略注释
                self._rules = [
                    line.strip()
                    for line in f.readlines()
                    if line.strip() and not line.startswith("#")
                ]
        else:
            # 默认规则
            self._rules = [
                "永不泄露用户隐私数据（password/secret/token 等）",
                "不无限循环调用 API（单次任务最多 10 次工具调用）",
                "不执行 rm -rf / 等高危系统命令",
                "不修改/etc 下的系统配置文件",
                "不通过 curl/wget 管道执行远程脚本"
            ]

    def _compile_patterns(self) -> None:
        """预编译正则表达式以提高性能。"""
        self._compiled_patterns = [
            (i, re.compile(pattern, re.IGNORECASE))
            for i, pattern in enumerate(self.HIGH_RISK_PATTERNS)
        ]

    def check(
        self, 
        tool_name: str, 
        params: Dict[str, Any], 
        context: Optional[Dict[str, Any]] = None
    ) -> GeneLockCheckResult:
        """
        校验工具调用是否违反 L0 基因锁。
        
        Args:
            tool_name: 工具名称
            params: 工具参数
            context: 上下文信息（可选），包含 call_count 等
            
        Returns:
            GeneLockCheckResult: 校验结果
        """
        # 规则 1: 检查高频 API 调用（防无限循环）
        if context and context.get("call_count", 0) >= 10:
            return GeneLockCheckResult(
                passed=False,
                reason="触发 L0 规则：单次任务工具调用次数达到上限（10 次）",
                rule_index=1
            )
        
        # 规则 2: 检查高危命令
        if tool_name in ["shell_exec", "run_command", "execute"]:
            command = params.get("command", "") or params.get("cmd", "")
            if isinstance(command, str):
                for pattern_idx, pattern in self._compiled_patterns:
                    if pattern.search(command):
                        return GeneLockCheckResult(
                            passed=False,
                            reason=f"触发 L0 规则：检测到高危命令 '{command[:50]}...'",
                            rule_index=pattern_idx + 2  # +2 因为是第 3 条规则开始
                        )
        
        # 规则 3: 检查隐私数据泄露
        if tool_name in ["write_file", "upload", "send_email", "http_post"]:
            content = ""
            for key in ["content", "data", "body", "text"]:
                if key in params and isinstance(params[key], str):
                    content = params[key].lower()
                    break
            
            if content:
                for keyword in self.PRIVACY_KEYWORDS:
                    if keyword in content:
                        return GeneLockCheckResult(
                            passed=False,
                            reason=f"触发 L0 规则：检测到敏感关键词 '{keyword}'",
                            rule_index=0
                        )
        
        # 规则 4: 语义规则匹配（简化版：关键词匹配）
        for i, rule in enumerate(self._rules):
            if self._matches_rule(tool_name, params, rule):
                return GeneLockCheckResult(
                    passed=False,
                    reason=f"触发 L0 规则：{rule}",
                    rule_index=i
                )
        
        # 通过所有校验
        return GeneLockCheckResult(passed=True)

    def _matches_rule(self, tool_name: str, params: Dict[str, Any], rule: str) -> bool:
        """
        简化的语义规则匹配。
        未来可接入小型本地模型进行更精确的语义理解。
        """
        rule_lower = rule.lower()
        
        # 检查规则中是否包含工具名
        if tool_name.lower() in rule_lower:
            return True
        
        # 检查参数中是否有规则禁止的内容
        for value in params.values():
            if isinstance(value, str) and value.lower() in rule_lower:
                return True
        
        return False

    def reload(self) -> None:
        """重新加载规则文件（支持热重载）。"""
        self._rules = []
        self._load_rules()
        self._compile_patterns()

    def get_summary(self) -> str:
        """获取规则摘要（用于注入 System Prompt）。"""
        if not self._rules:
            return "无活跃基因锁规则"
        return "\n".join(f"{i+1}. {rule}" for i, rule in enumerate(self._rules[:5]))
    
    def get_rules_count(self) -> int:
        """获取当前规则数量。"""
        return len(self._rules)
