"""HXR Logger - Helix Execution Record for cognitive audit."""

import json
import hashlib
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict, Any


class HXRLogger:
    """Helix Execution Record logger with tamper-proof hash chain."""

    def __init__(self, hxr_dir: str = "./memory_dag/sessions"):
        self.hxr_dir = Path(hxr_dir)
        self.hxr_dir.mkdir(parents=True, exist_ok=True)
        self._last_hash: Optional[str] = None
        self._session_file: Optional[Path] = None

    def start_session(self, session_id: str) -> None:
        """Start a new session and initialize hash chain."""
        self._session_file = self.hxr_dir / f"{session_id}.jsonl"
        if self._session_file.exists():
            with open(self._session_file, "r") as f:
                lines = f.readlines()
                if lines:
                    last_record = json.loads(lines[-1])
                    self._last_hash = last_record.get("hash_curr")

    def write(self, record: Dict[str, Any]) -> None:
        """Write an HXR record with tamper-proof hash."""
        if not self._session_file:
            raise ValueError("Session not started. Call start_session() first.")

        # Fill required fields
        record.setdefault("ts", datetime.now().isoformat())

        # Compute tamper-proof hash
        record["hash_prev"] = self._last_hash
        hash_input = json.dumps(record, sort_keys=True, ensure_ascii=False)
        record["hash_curr"] = hashlib.sha256(hash_input.encode()).hexdigest()

        # Append to file
        with open(self._session_file, "a") as f:
            f.write(json.dumps(record, ensure_ascii=False) + "\n")

        self._last_hash = record["hash_curr"]
