import uuid
import structlog
from contextvars import ContextVar

_trace_id_var: ContextVar[str] = ContextVar("trace_id", default="")


def generate_trace_id() -> str:
    return uuid.uuid4().hex


def set_trace_id(trace_id: str) -> None:
    _trace_id_var.set(trace_id)
    structlog.contextvars.bind_contextvars(trace_id=trace_id)


def get_trace_id() -> str:
    return _trace_id_var.get()


def generate_epoch_id() -> str:
    return f"epoch_{uuid.uuid4().hex[:12]}"
