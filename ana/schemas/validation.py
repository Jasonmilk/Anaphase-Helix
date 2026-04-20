from pydantic import BaseModel
from typing import Literal, Optional, Dict


class ValidationResult(BaseModel):
    passed: bool
    reason: Optional[Literal["split_brain", "munchausen", "alignment_low", "doc_mismatch"]] = None
    action: Literal["proceed", "downgrade", "freeze", "report", "regenerate"]
    flags: dict[str, bool] = {}
    mismatch_report: Optional[Dict] = None
