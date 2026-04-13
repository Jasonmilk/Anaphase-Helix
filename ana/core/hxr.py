"""HXR (Helix Execution Record) logger for structured session logging."""

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class HXREntry:
    """Single HXR log entry representing one step in the Agent Loop."""

    session_id: str
    loop_id: int
    action: str
    params: dict[str, Any]
    result: str  # "success" or "failed"
    reasoning: str | None
    ts: str
    tool_result: dict[str, Any] | None = None
    error: str | None = None


class HXRLogger:
    """Structured logger for Helix Execution Records.

    Records each step of the Agent Loop to enable trace replay and audit.
    Logs are stored as JSONL files organized by session ID.
    """

    def __init__(self, hxr_dir: str = "./memory_dag/sessions") -> None:
        """Initialize HXR logger.

        Args:
            hxr_dir: Directory to store session logs
        """
        self.hxr_dir = Path(hxr_dir)
        self._ensure_dir()

    def _ensure_dir(self) -> None:
        """Ensure the HXR directory exists."""
        self.hxr_dir.mkdir(parents=True, exist_ok=True)

    def _get_session_path(self, session_id: str) -> Path:
        """Get the path to a session's log file.

        Args:
            session_id: Unique session identifier

        Returns:
            Path to the session's JSONL log file
        """
        return self.hxr_dir / f"{session_id}.jsonl"

    def write(self, entry: dict[str, Any]) -> None:
        """Write an HXR entry to the session log.

        Args:
            entry: Dictionary containing HXR entry fields
        """
        session_id = entry.get("session_id", "unknown")
        session_path = self._get_session_path(session_id)

        with open(session_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")

    def read_session(self, session_id: str) -> list[dict[str, Any]]:
        """Read all entries for a session.

        Args:
            session_id: Unique session identifier

        Returns:
            List of HXR entries in chronological order
        """
        session_path = self._get_session_path(session_id)
        if not session_path.exists():
            return []

        entries = []
        with open(session_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    entries.append(json.loads(line))

        return entries

    def get_step(self, session_id: str, step: int) -> dict[str, Any] | None:
        """Get a specific step from a session.

        Args:
            session_id: Unique session identifier
            step: Step number (1-indexed)

        Returns:
            HXR entry for the specified step, or None if not found
        """
        entries = self.read_session(session_id)
        for entry in entries:
            if entry.get("loop_id") == step:
                return entry
        return None

    def list_sessions(self) -> list[str]:
        """List all available session IDs.

        Returns:
            List of session IDs
        """
        sessions = []
        if self.hxr_dir.exists():
            for file in self.hxr_dir.glob("*.jsonl"):
                sessions.append(file.stem)
        return sorted(sessions)

    def clear_session(self, session_id: str) -> bool:
        """Clear a session's log file.

        Args:
            session_id: Unique session identifier

        Returns:
            True if session was cleared, False if it didn't exist
        """
        session_path = self._get_session_path(session_id)
        if session_path.exists():
            session_path.unlink()
            return True
        return False
