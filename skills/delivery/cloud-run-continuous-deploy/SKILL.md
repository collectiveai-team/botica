---
name: cloud-run-continuous-deploy
description: Prepare a repo with a docker-compose.yml for continuous deployment to Google Cloud Run — dev on merge to dev, prod on merge to main, and a resettable integration environment from an immutable PR review tag. Use when wiring a repo to Cloud Run, when a deploy workflow hands credentials to an untrusted ref, when review deployments need isolation from prod, or when secrets must reach GitHub Environments without passing through an agent.
---

# Cloud Run continuous deploy

Procedure and invariants land in Task 9.
