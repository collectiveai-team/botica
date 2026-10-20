"""The process that serves the deployments of the package and runs their flow runs.

It is not a worker: a worker polls a work pool and starts infrastructure for each run, and this
process runs each flow run itself, as a subprocess.
"""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING

from prefect import aserve

from app.flows.documents import summarize_documents

if TYPE_CHECKING:
    from prefect.deployments.runner import RunnerDeployment


async def deployments() -> list[RunnerDeployment]:
    """Return the deployments that this process serves."""
    return [await summarize_documents.ato_deployment(name="summarize-documents")]


async def main() -> None:
    """Register the deployments and run their flow runs until the process stops."""
    await aserve(*await deployments())


if __name__ == "__main__":
    asyncio.run(main())
