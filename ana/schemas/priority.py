from pydantic import BaseModel, Field
from enum import Enum


class IntentCategory(str, Enum):
    CHAT = "chat"
    KNOWLEDGE_RETRIEVAL = "knowledge_retrieval"
    SOCIAL_GRAPH_READ = "social_graph_read"
    TASK = "task"


class PriorityAssessment(BaseModel):
    priority_score: float = Field(..., ge=0.0, le=100.0)
    intent_category: IntentCategory
