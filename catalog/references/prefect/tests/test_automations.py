"""The automation of the topic is created on the server with its trigger and its webhook."""

from __future__ import annotations

import asyncio

from prefect.automations import Automation
from prefect.blocks.webhook import Webhook
from prefect.events.actions import CallWebhook
from prefect.events.schemas.automations import EventTrigger
from prefect.events.schemas.events import ResourceSpecification

from app.automations import notify_failed_summaries


def test_the_automation_calls_the_webhook_when_a_run_of_the_summaries_fails_or_crashes():
    async def created() -> tuple[Automation, Webhook]:
        automation = await notify_failed_summaries("https://hooks.example.com/summaries")
        return await Automation.aread(id=automation.id), await Webhook.aload("summaries-failed")

    automation, webhook = asyncio.run(created())

    assert isinstance(automation.trigger, EventTrigger)
    assert automation.trigger.expect == {"prefect.flow-run.Failed", "prefect.flow-run.Crashed"}
    related = automation.trigger.match_related
    assert isinstance(related, ResourceSpecification)
    assert related.root == {
        "prefect.resource.role": "flow",
        "prefect.resource.name": "summarize-documents",
    }
    [action] = automation.actions
    assert isinstance(action, CallWebhook)
    assert webhook.url.get_secret_value() == "https://hooks.example.com/summaries"
