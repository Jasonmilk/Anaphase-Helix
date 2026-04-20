from pydantic import BaseModel
from typing import List, Dict
from datetime import datetime


class MilestoneSnapshot(BaseModel):
    snapshot_id: str
    created_at: datetime
    epoch_id: str
    confirmed_docs: Dict[str, str]  # Document name → version
    completed_modules: List[str]
    pending_tasks_dag_ref: str      # Node ID in L2 DAG for pending tasks
    active_contracts: List[str]     # Active contract lock rules
