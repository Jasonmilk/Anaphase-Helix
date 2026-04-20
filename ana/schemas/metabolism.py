from datetime import datetime
from pydantic import BaseModel


class MetabolismState(BaseModel):
    used_tokens: int = 0
    budget_total: int
    epoch_start_time: datetime
    max_duration_seconds: int
    working_memory_items: int = 0
    cognitive_overload_line: int
    fatigue_line: float
    apoptosis_line: float
