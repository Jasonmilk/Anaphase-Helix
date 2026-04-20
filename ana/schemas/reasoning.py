from pydantic import BaseModel
from typing import Optional, List, Dict, Any


class ReasoningRequest(BaseModel):
    task: str
    system_prompt: str
    working_memory: List[str] = []
    episodic_memory: List[Dict[str, Any]] = []
    metabolism_state: Dict[str, Any] = {}
    trace_id: str


class ReasoningDraft(BaseModel):
    reasoning: Optional[str] = None
    tool_call: Optional[Dict[str, Any]] = None
    final_reply: Optional[str] = None
