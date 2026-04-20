import click
from importlib.metadata import version, PackageNotFoundError

from .run import run
from .trace import trace
from .stats import stats


try:
    __version__ = version("helix-ana")
except PackageNotFoundError:
    __version__ = "unknown"


@click.group()
@click.version_option(version=__version__)
def cli():
    """Anaphase-Helix CLI: Execution orchestration core of Helix ecosystem."""
    pass


cli.add_command(run)
cli.add_command(trace)
cli.add_command(stats)
