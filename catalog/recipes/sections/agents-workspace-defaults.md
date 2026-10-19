## Repo Workspace Defaults

- Use `.scratch/` for temporary plans, issue drafts, and disposable notes.
- Use `.tmp/` for generated local artifacts that should not be committed.
- Use `.worktrees/` for local Git worktrees when needed.
- Use `.journals/` for local/private session journals when using the `journalist` skill.
- Preserve unrelated user changes and avoid destructive Git commands unless explicitly requested.

### Resource Limits For Heavy Commands

Run computationally expensive commands (heavy builds, full test suites, data jobs, memory-hungry tooling) under `systemd-run` so a runaway process cannot exhaust the machine:

- Cap memory with `MemoryMax` (hard limit; process is killed if exceeded) and disable swap thrashing with `MemorySwapMax=0`.
- Cap CPU with `CPUQuota` (e.g. `400%` = up to 4 cores).
- Use a transient user scope (no root needed). If `systemd-run` is unavailable, fall back to `ulimit -v` for a memory cap.

Measure and log wall time and peak memory with `/usr/bin/time -v`, appending to `.tmp/logs/<command>/<date>_wall-stats.log`. Read the latest log before re-running to estimate limits instead of guessing:

```bash
cmd=build; ts=$(date +%Y-%m-%dT%H-%M-%S)
log=.tmp/logs/$cmd/${ts}_wall-stats.log; mkdir -p ".tmp/logs/$cmd"
systemd-run --user --scope -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=400% \
  /usr/bin/time -v -o "$log" <command>
```

Run long commands in the background with a saved PID so they are not tied to the agent tool timeout, and delegate polling to a cheap monitor sub-agent (e.g. Haiku) that runs a `while kill -0 <pid>; do sleep <n>; done` loop and reports only the final exit status, output tail, and stats log path. This keeps the main session from being poisoned by repeated status checks.

```bash
( systemd-run --user --scope -p MemoryMax=4G -p CPUQuota=400% \
    /usr/bin/time -v -o "$log" <command> ) >".tmp/logs/$cmd/${ts}.out" 2>&1 &
echo $! > ".tmp/logs/$cmd/${ts}.pid"
```

Scripts we write ourselves must emit periodic progress to the output log (current step, counts, percent, or heartbeat with a timestamp) so the monitor can tell what they are doing. Flush each line (e.g. Python `print(..., flush=True)` or `PYTHONUNBUFFERED=1`) so progress appears live. The monitor always `tail`s the latest lines of the `.out` log — never reads the full log — to avoid poisoning the session with bulk output.

A consistently too-expensive step is a signal of poor code that needs optimization, not a reason to keep raising the limits — flag it.
