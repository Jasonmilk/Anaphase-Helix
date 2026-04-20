import structlog


def configure_logging() -> None:
    structlog.configure(
        processors=[
            structlog.contextvars.merge_contextvars,
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.JSONRenderer(),
        ]
    )


logger = structlog.get_logger()


def log_hxr(step: str, action: str, duration_ms: float, **kwargs) -> None:
    logger.info("hxr", step=step, action=action, duration_ms=duration_ms, **kwargs)
