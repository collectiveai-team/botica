A Prefect flow is thin orchestration over domain code. The API starts flow runs by the name of their
deployment and imports no flow, and a notification of a failed run is an automation of the server.

## What to read

- `summarize_documents` is an async flow. It logs with `get_run_logger()` and the format of the
  standard library (`logger.info("summarizing %d documents", n)`): that logger takes no keyword
  pairs. It runs the independent tasks in an `asyncio.TaskGroup`, which cancels the others when one
  fails, and publishes a Markdown artifact. An artifact key has lowercase letters, numbers and dashes
  only; Prefect rejects any other. An artifact stays in the database of Prefect, so it carries no
  secret and no personal data.
- `summarize` is domain code. It logs with the house `get_logger`, and its events reach the logs of
  the run when the process sets `PREFECT_LOGGING_EXTRA_LOGGERS=app` (the name of the package) and
  `PREFECT_LOGGING_ROOT_LEVEL=INFO`: Prefect gives the logger of the package its handlers, not a
  level, and its root logger is at `WARNING`. The process sets `ENV=prod`, so the events are JSON
  and not the colored console lines.
- `deploy.py` declares the deployments in Python and serves them with `aserve`. That process runs
  each flow run as a subprocess; it is not a worker. When each run needs its own infrastructure, use
  `flow.deploy()` with a work pool and a worker instead. A repository uses one of the two, and does
  not also keep a `prefect.yaml`.
- `runs.py` is what the API uses. `start_summaries` calls `arun_deployment(..., timeout=0)`, which
  creates the flow run and returns at once. `wait_or_cancel` waits with `wait_for_flow_run` and, past
  the deadline, sets the run to `Cancelling()` so that the process that runs it stops it. The module
  names the deployment and imports no flow, so the API does not load the code of the flows.
- `notify_failed_summaries` creates an automation whose trigger is a failed or crashed run of the
  flow and whose action calls a webhook block. The server runs it, so it fires even when the
  process of the run dies, which a state hook (`on_failure`, `on_crashed`) cannot promise.
- The tests run on a temporary server (`prefect_test_harness`).

## What to adapt

- The flow, the domain code and the names are one example.
- The address of the server is `PREFECT_API_URL` in the environment of each process, read by
  Prefect itself. The address of the webhook and the deadline of a run are settings of the package.
- Persist the result of a flow run only when another process reads it, and say where it is stored.
  When the API needs what a run produced, the flow writes it where the API reads it, such as its
  database.
