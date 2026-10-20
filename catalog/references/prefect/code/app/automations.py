"""The automation that calls a webhook when a flow run of the summaries fails or crashes.

The server runs it, so it fires even when the process of the flow run dies, which a state hook of
the flow cannot promise.
"""

from __future__ import annotations

from prefect.automations import Automation
from prefect.blocks.webhook import Webhook
from prefect.events.actions import CallWebhook
from prefect.events.schemas.automations import EventTrigger, Posture
from pydantic import SecretStr

PAYLOAD = '{"flow_run": "{{ flow_run.name }}", "state": "{{ flow_run.state.name }}"}'


async def notify_failed_summaries(webhook_url: str) -> Automation:
    """Create the automation, and the webhook block it calls, on the server of Prefect."""
    webhook = Webhook(method="POST", url=SecretStr(webhook_url))
    block_id = await webhook.asave("summaries-failed", overwrite=True)
    automation = Automation(
        name="notify-failed-summaries",
        trigger=EventTrigger(
            expect={"prefect.flow-run.Failed", "prefect.flow-run.Crashed"},
            match={"prefect.resource.id": "prefect.flow-run.*"},
            match_related={
                "prefect.resource.role": "flow",
                "prefect.resource.name": "summarize-documents",
            },
            posture=Posture.Reactive,
            threshold=1,
        ),
        actions=[CallWebhook(block_document_id=block_id, payload=PAYLOAD)],
    )
    return await automation.acreate()
