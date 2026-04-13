import yaml
import jsonschema
from typing import Dict, Any, Optional, List
from pydantic import BaseModel

class ToolHandler(BaseModel):
    executor: str
    command: str
    timeout: int = 30
    output_mapping: Optional[Dict[str, str]] = None

class ToolSchema(BaseModel):
    name: str
    description: str
    aliases: Optional[List[str]] = None
    parameters: Dict[str, Any]
    handler: ToolHandler
    permissions: List[str]

class ToolRegistry:
    def __init__(self, config_path: str = "config/tools.yaml"):
        self.tools: Dict[str, ToolSchema] = {}
        self._load_defaults()

    def _load_defaults(self):
        default_tools = [
            ToolSchema(
                name="ana_kb_fetch",
                description="从知识库 DAG 调取节点详情。可以通过 node_id 精确获取，也可以通过 query 语义搜索。",
                aliases=["fetch"],
                parameters={
                    "type": "object",
                    "properties": {
                        "node_id": {"type": "string"},
                        "query": {"type": "string"},
                        "mode": {"type": "string", "enum": ["summary", "full"], "default": "summary"}
                    },
                    # 不设 required，允许任意组合
                },
                handler=ToolHandler(
                    executor="ana_cli",
                    command="echo '{\"id\": \"$node_id$query\", \"title\": \"快速排序\", \"summary\": \"分治算法，平均时间复杂度 O(n log n)\"}'",
                    timeout=5,
                    output_mapping={"format": "json"}
                ),
                permissions=["read"]
            ),
            ToolSchema(
                name="ana_finish",
                description="任务完成时调用此工具，以优雅结束 Agent Loop。",
                aliases=["finish", "plan"],
                parameters={
                    "type": "object",
                    "properties": {
                        "summary": {"type": "string", "description": "任务完成摘要"},
                        "confidence": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.9}
                    },
                },
                handler=ToolHandler(
                    executor="ana_cli",
                    command="echo '{\"status\": \"completed\", \"message\": \"Task finished successfully\"}'",
                    timeout=10,
                    output_mapping={"format": "json"}
                ),
                permissions=["execute"]
            ),
            ToolSchema(
                name="update_social_graph",
                description="更新社交图谱中某个实体的属性。例如，添加别名、年龄、关系状态等。",
                parameters={
                    "type": "object",
                    "properties": {
                        "entity_id": {"type": "string", "description": "要更新的实体 ID，如 'person_xiaoming'"},
                        "updates": {"type": "object", "description": "包含要更新字段及其值的 JSON 对象，如 {'aliases': ['陈小明'], 'age': 34}"}
                    },
                    "required": ["entity_id", "updates"]
                },
                handler=ToolHandler(
                    executor="external_cli",
                    command="./scripts/update_social_graph.sh '{entity_id}' '{updates}'",
                    timeout=10,
                    output_mapping={"format": "json"}
                ),
                permissions=["write"]
            ),
            ToolSchema(
                name="query_social_graph",
                description="Retrieve information about a person from the social graph.",
                parameters={
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Name or alias of the person"}
                    },
                    "required": ["name"]
                },
                handler=ToolHandler(
                    executor="external_cli",
                    command="yq eval '.entities[] | select(.name == \"{name}\" or .aliases[] == \"{name}\")' knowledge_base/social_graph.md",
                    timeout=5,
                    output_mapping={"format": "json"}
                ),
                permissions=["read"]
            )
        ]
        for tool in default_tools:
            self.tools[tool.name] = tool
            for alias in (tool.aliases or []):
                self.tools[alias] = tool

    def get(self, name: str) -> Optional[ToolSchema]:
        return self.tools.get(name)

    def validate(self, name: str, params: Dict[str, Any]) -> bool:
        tool = self.get(name)
        if not tool:
            return False
        try:
            jsonschema.validate(params, tool.parameters)
            return True
        except jsonschema.ValidationError:
            # 对于 ana_finish，放宽校验
            if name in ("ana_finish", "finish", "FINISH"):
                return True
            return False
