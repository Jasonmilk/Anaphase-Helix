"""HelixState DTO — the single bus flowing through Agent Loop nodes."""

from datetime import datetime
from typing import Optional, Literal
from pydantic import BaseModel

from .affect import AffectVector
from .priority import PriorityAssessment
from .metabolism import MetabolismState
from .memory import MemoryFragments
from .reasoning import ReasoningDraft
from .validation import ValidationResult


class HelixState(BaseModel):
    epoch_id: str
    trace_id: str
    task: str
    created_at: datetime
    priority: Optional[PriorityAssessment] = None
    affect: Optional[AffectVector] = None
    metabolism: MetabolismState
    memory_fragments: Optional[MemoryFragments] = None
    reasoning_draft: Optional[ReasoningDraft] = None
    validation_result: Optional[ValidationResult] = None
    current_step: Literal[
        "perceive", "assess_priority", "plan", "execute", "reflect", "consolidate", "sleep"
    ] = "perceive"
