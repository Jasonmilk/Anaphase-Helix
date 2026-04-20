import httpx
from tenacity import retry, stop_after_attempt, wait_exponential, retry_if_exception, before_sleep_log
import structlog

logger = structlog.get_logger()


def is_retryable(exception: Exception) -> bool:
    if isinstance(exception, httpx.TimeoutException):
        return True
    if isinstance(exception, httpx.HTTPStatusError):
        return exception.response.status_code in (429, 502, 503, 504)
    if isinstance(exception, httpx.ConnectError):
        return True
    return False


llm_retry = retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=2, max=10),
    retry=retry_if_exception(is_retryable),
    before_sleep=before_sleep_log(logger, "WARNING"),
    reraise=True,
)
