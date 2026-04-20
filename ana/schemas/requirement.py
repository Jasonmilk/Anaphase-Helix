from pydantic import BaseModel, Field
from typing import List, Optional
from enum import Enum


class Priority(str, Enum):
    P0 = "p0"  # Hard constraint, must be implemented
    P1 = "p1"  # Core functionality
    P2 = "p2"  # Enhancement


class RequirementItem(BaseModel):
    req_id: str = Field(..., description="Unique requirement ID, format REQ-XXX")
    description: str
    acceptance_criteria: List[str]  # Quantifiable acceptance criteria
    priority: Priority
    parent_req_id: Optional[str] = None  # Supports hierarchical decomposition


class RequirementSpec(BaseModel):
    version: str = Field(..., description="Semantic version, e.g. 1.0.0")
    project_name: str
    items: List[RequirementItem]
