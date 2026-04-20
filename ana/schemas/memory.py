"""Memory contracts: Queries and retrieved fragments from Hippocampus."""

from pydantic import BaseModel, Field
from typing import List, Optional


class MemoryQuery(BaseModel):
    query: str
    intent: str
    limit: int = 5
    layer_filter: Optional[List[str]] = None
    trace_id: str


class MemoryFragment(BaseModel):
    id: str
    title: str
    summary: str
    confidence: float
    initial_impact: float = Field(default=0.0)
    heat: float = Field(default=1.0)
    is_anchored: bool = Field(default=False)


class MemoryFragments(BaseModel):
    nodes: List[MemoryFragment]
