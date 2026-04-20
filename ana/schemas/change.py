from pydantic import BaseModel
from typing import List, Dict


class ChangeImpactReport(BaseModel):
    change_id: str
    reason: str
    affected_req_ids: List[str]
    affected_design_docs: List[str]
    affected_code_files: List[str]
    affected_test_cases: List[str]
    recommended_doc_version_bumps: Dict[str, str]  # Document path → new version
