"""Structured logging configuration — HXR JSONL format."""

import logging
import structlog
from pathlib import Path


def configure_logging() -> None:
    """
    Configure structlog to use standard logging module.

    This allows both console output (via StreamHandler) and file persistence
    (via FileHandler added by `add_file_logger`) to work seamlessly.
    """
    # Configure standard logging with a console handler
    logging.basicConfig(
        level=logging.INFO,
        format="%(message)s",               # Raw message, structlog already formats JSON
        handlers=[logging.StreamHandler()]  # Console output
    )

    # Configure structlog to use standard logging factory
    structlog.configure(
        processors=[
            structlog.contextvars.merge_contextvars,
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.JSONRenderer(),
        ],
        wrapper_class=structlog.make_filtering_bound_logger(logging.INFO),
        context_class=dict,
        logger_factory=structlog.stdlib.LoggerFactory(),  # Use stdlib logging
        cache_logger_on_first_use=True,
    )


def add_file_logger(file_path: Path) -> None:
    """
    Attach a file handler to the root logger.

    All structlog output (already JSON-formatted) will be appended
    to the specified file. The parent directory is created if missing.
    The handler is configured to write through immediately to prevent
    log loss on process exit.

    Args:
        file_path: Path to the JSONL file.
    """
    file_path.parent.mkdir(parents=True, exist_ok=True)

    file_handler = logging.FileHandler(file_path, encoding="utf-8")
    file_handler.setLevel(logging.INFO)
    file_handler.setFormatter(logging.Formatter("%(message)s"))

    # Force unbuffered writes to ensure logs are persisted even on abrupt exit
    if hasattr(file_handler.stream, "reconfigure"):
        file_handler.stream.reconfigure(write_through=True)

    root_logger = logging.getLogger()
    root_logger.addHandler(file_handler)


# Global logger instance
logger = structlog.get_logger()


def log_hxr(step: str, action: str, duration_ms: float, **kwargs) -> None:
    """
    Write a structured HXR audit record.

    trace_id and epoch_id are automatically injected via contextvars.
    """
    logger.info(
        "hxr",
        step=step,
        action=action,
        duration_ms=duration_ms,
        **kwargs,
    )
