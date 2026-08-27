# Cloud Run Continuous Deploy Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `cloud-run-continuous-deploy`, a house-baseline skill that prepares any repo with a `docker-compose.yml` for three-trigger continuous deployment to Google Cloud Run.

**Architecture:** A skill directory under a new `skills/delivery/` category holding prose (`SKILL.md`, `reference.md`), one read-only inventory script, and ten templates the agent adapts per repo. The twelve security invariants from the spec are pinned by a static test suite that reads the template text, so a future edit that breaks one fails CI rather than a production deploy.

**Tech Stack:** Python 3.11+ (stdlib only), bash, GitHub Actions, `gcloud`, `gh`, `varlock`, nginx, pytest, ruff.

**Spec:** `docs/superpowers/specs/2026-08-26-cloud-run-continuous-deploy-design.md`

## Global Constraints

- Skill name is exactly `cloud-run-continuous-deploy`; directory is `skills/delivery/cloud-run-continuous-deploy/`.
- GitHub Environment names are fixed: `dev`, `prod`, `integration`. Branch names are configurable and default to `dev` and `main`.
- Templates use `{{DOUBLE_BRACE}}` placeholders. No other substitution syntax.
- No template may contain `iam service-accounts keys create` (invariant 12).
- Python in templates is stdlib-only — the target repo may have no dependencies installed.
- Shell scripts using `wait -n` or `${!var}` indirect expansion must use `#!/usr/bin/env bash`, never `sh`.
- No secret value is ever echoed, logged, written to a file, or passed in argv.
- Every new local skill must satisfy `tests/test_skill_catalog.py` unchanged.
- Commit messages follow the repo's Conventional Commits usage (`feat(skills):`, `test(skills):`, `docs(skills):`).

---

### Task 1: Register the skill and its skeleton

The existing catalog tests already encode every registration point. This task drives them from red to green, which is why it comes first: it proves the registration surface is complete before any content exists.

**Files:**
- Modify: `scaffolding/skills.py:92`
- Create: `skills/delivery/cloud-run-continuous-deploy/SKILL.md`
- Create: `skills/delivery/cloud-run-continuous-deploy/agents/openai.yaml`
- Modify: `README.md:205`, `README.md:214`, `README.md:253`
- Modify: `guide.md:289`, `guide.md:293`, `guide.md:303`
- Modify: `skills/productivity/ask-user/SKILL.md`
- Test: `tests/test_skill_catalog.py` (existing, unmodified)

**Interfaces:**
- Consumes: nothing.
- Produces: the skill directory root `skills/delivery/cloud-run-continuous-deploy/`, referenced by every later task.

- [ ] **Step 1: Add the skill to the baseline to turn the catalog tests red**

In `scaffolding/skills.py`, replace line 92:

```python
LOCAL_SKILLS = [
    "ask-user",
    "journalist",
    "handoff",
    "test-smell-review",
    "unattended-issue-driver",
    "cloud-run-continuous-deploy",
]
```

- [ ] **Step 2: Verify RED**

Run: `uv run pytest tests/test_skill_catalog.py -v`

Expected: four failures — `test_local_skills_exist_on_disk` ("has no SKILL.md under skills/*/"), `test_user_invoked_local_skills_carry_codex_policy`, `test_docs_carry_the_exact_local_skill_list[README.md]` and `[guide.md]`, and `test_ask_user_routes_every_installed_skill`.

- [ ] **Step 3: Create the skill skeleton**

`skills/delivery/cloud-run-continuous-deploy/SKILL.md` — frontmatter plus a one-line body. The full procedure lands in Task 9; this is the minimum that satisfies the catalog.

```markdown
---
name: cloud-run-continuous-deploy
description: Prepare a repo with a docker-compose.yml for continuous deployment to Google Cloud Run — dev on merge to dev, prod on merge to main, and a resettable integration environment from an immutable PR review tag. Use when wiring a repo to Cloud Run, when a deploy workflow hands credentials to an untrusted ref, when review deployments need isolation from prod, or when secrets must reach GitHub Environments without passing through an agent.
---

# Cloud Run continuous deploy

Procedure and invariants land in Task 9.
```

`skills/delivery/cloud-run-continuous-deploy/agents/openai.yaml`:

```yaml
interface:
  display_name: "Cloud Run Continuous Deploy"
  short_description: "Wire a docker-compose repo to Cloud Run with dev, prod and review environments"
```

Note the SKILL.md frontmatter has **no** `disable-model-invocation: true`, so `agents/openai.yaml` must **not** carry `allow_implicit_invocation: false`. `test_user_invoked_local_skills_carry_codex_policy` asserts those two agree.

- [ ] **Step 4: Update the install command in both docs**

`README.md:205` and `guide.md:293` — append the name to the `--skill` list:

```bash
npx skills add collectiveai-team/scaffolding --agent opencode --yes --skill ask-user journalist handoff test-smell-review unattended-issue-driver cloud-run-continuous-deploy
```

`README.md:214` and `guide.md:303` — the same list in the `npx skills add .` local-install sentence:

```
`npx skills add . --agent opencode --yes --skill ask-user journalist handoff test-smell-review unattended-issue-driver cloud-run-continuous-deploy --full-depth`.
```

`guide.md:289` — the prose listing the local skills:

```
`test-smell-review`, `unattended-issue-driver`, `cloud-run-continuous-deploy`), and varlock:
```

`test_docs_carry_the_exact_local_skill_list` asserts `" ".join(LOCAL_SKILLS)` appears verbatim, so the order above must match `LOCAL_SKILLS` exactly.

- [ ] **Step 5: Add the README catalog entry**

After the `skills/productivity/unattended-issue-driver` bullet at `README.md:253`:

```markdown
- `skills/delivery/cloud-run-continuous-deploy` — prepare a docker-compose repo for
  continuous deployment to Cloud Run: dev on merge to `dev`, prod on merge to `main`,
  and a resettable integration environment deployed from an immutable PR review tag.
```

- [ ] **Step 6: Add the ask-user router entry**

In `skills/productivity/ask-user/SKILL.md`, next to the `/unattended-issue-driver` line:

```markdown
- `/cloud-run-continuous-deploy`: wire a repo with a `docker-compose.yml` to Google Cloud Run, with separate dev, prod and review-tag environments. Reach for it when a deploy hands credentials to an untrusted ref, when review deployments can reach production data, or when secrets need to get into GitHub Environments without an agent seeing them.
```

The test matches on the substring `/cloud-run-continuous-deploy`, so the leading slash is required.

- [ ] **Step 7: Verify GREEN and commit**

```bash
uv run pytest tests/test_skill_catalog.py -v
git add scaffolding/skills.py skills/delivery README.md guide.md skills/productivity/ask-user/SKILL.md
git commit -m "feat(skills): register cloud-run-continuous-deploy"
```

---

### Task 2: Review tag parser and validator template

**Files:**
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/review_tag.py`
- Test: `tests/test_review_tag_template.py`

**Interfaces:**
- Consumes: the skill directory from Task 1.
- Produces: `parse_review_tag(tag: str) -> ReviewTag`, `validate(tag: str, repo: str, base: str, run: Callable[[list[str]], str]) -> tuple[ReviewTag, str]` returning `(parsed, head_sha)`, and the frozen dataclass `ReviewTag(pr_number: int, short_sha: str)`. The CLI is `python3 scripts/review_tag.py validate --tag TAG --repo OWNER/REPO --base BRANCH [--emit-github-output]`. Tasks 6 and 9 call that CLI by exactly that name.

- [ ] **Step 1: Write the failing tests**

`tests/test_review_tag_template.py`:

```python
"""The review-tag template ships verbatim into target repos, so it is tested here.

It is loaded by path rather than imported, because `skills/` is not a package and
the file's runtime home is another repository entirely.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import pytest

TEMPLATE = (
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/templates/review_tag.py"
)


def _load():
    spec = importlib.util.spec_from_file_location("review_tag_template", TEMPLATE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


review_tag = _load()


def _pr_payload(**overrides):
    payload = {
        "state": "open",
        "draft": False,
        "base": {"ref": "dev"},
        "head": {"sha": "abcdef123456789012345678901234567890abcd"},
    }
    payload.update(overrides)
    return payload


def _runner(payload, git_sha="abcdef123456789012345678901234567890abcd"):
    def run(cmd):
        if cmd[0] == "gh":
            return json.dumps(payload)
        if cmd[0] == "git":
            return git_sha + "\n"
        raise AssertionError(f"unexpected command: {cmd}")

    return run


def test_parse_review_tag():
    parsed = review_tag.parse_review_tag("review/pr-123/abcdef123456")
    assert parsed.pr_number == 123
    assert parsed.short_sha == "abcdef123456"


@pytest.mark.parametrize(
    "tag",
    [
        "review/123/abcdef1",
        "review/pr-x/abcdef1",
        "review/pr-1/nope",
        "review/pr-0/abcdef1",
        "review/pr-1/ABCDEF1",
        "refs/tags/review/pr-1/abcdef1",
    ],
)
def test_rejects_invalid_review_tag(tag):
    with pytest.raises(ValueError, match="review/pr"):
        review_tag.parse_review_tag(tag)


def test_validate_accepts_open_pr_at_tagged_head():
    parsed, head_sha = review_tag.validate(
        "review/pr-7/abcdef123456", "org/repo", "dev", run=_runner(_pr_payload())
    )
    assert parsed.pr_number == 7
    assert head_sha == "abcdef123456789012345678901234567890abcd"


def test_validate_rejects_stale_tag():
    run = _runner(_pr_payload(), git_sha="0000000000000000000000000000000000000000")
    with pytest.raises(ValueError, match="does not point at"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_closed_pr():
    run = _runner(_pr_payload(state="closed"))
    with pytest.raises(ValueError, match="not open"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_draft_pr():
    run = _runner(_pr_payload(draft=True))
    with pytest.raises(ValueError, match="draft"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_wrong_base():
    run = _runner(_pr_payload(base={"ref": "main"}))
    with pytest.raises(ValueError, match="base branch"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_short_sha_that_is_not_a_head_prefix():
    run = _runner(_pr_payload(head={"sha": "9999999999999999999999999999999999999999"}))
    with pytest.raises(ValueError, match="head"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_malformed_api_response():
    def run(cmd):
        return "not json" if cmd[0] == "gh" else "abc\n"

    with pytest.raises(ValueError, match="could not read pull request"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)
```

- [ ] **Step 2: Verify RED**

Run: `uv run pytest tests/test_review_tag_template.py -v`

Expected: collection error — the template file does not exist, so `spec_from_file_location` returns a spec whose loader raises `FileNotFoundError`.

- [ ] **Step 3: Write the template**

`skills/delivery/cloud-run-continuous-deploy/templates/review_tag.py`:

```python
"""Parse and validate immutable PR review tags.

A review tag names exactly one pull-request head:

    review/pr-<number>/<short-sha>

Validation is not a formality. Anyone who can push a tag can create one, and the
trusted deployment workflow uses the result to choose which commit to build. It
must therefore prove against both the GitHub API and git that the tag points at
the head of an open, non-draft pull request targeting the trusted base branch.

Stdlib only: this file ships into repositories that may install nothing.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from typing import Callable

TAG_PATTERN = re.compile(r"^review/pr-([1-9][0-9]*)/([0-9a-f]{7,40})$")


@dataclass(frozen=True)
class ReviewTag:
    pr_number: int
    short_sha: str


def parse_review_tag(tag: str) -> ReviewTag:
    match = TAG_PATTERN.match(tag)
    if match is None:
        raise ValueError(
            f"not a review/pr tag: {tag!r}; expected review/pr-<number>/<short-sha> "
            "with a lowercase hex sha of 7 to 40 characters"
        )
    return ReviewTag(pr_number=int(match.group(1)), short_sha=match.group(2))


def _run(cmd: list[str]) -> str:
    return subprocess.run(cmd, capture_output=True, text=True, check=True).stdout


def validate(
    tag: str,
    repo: str,
    base: str,
    run: Callable[[list[str]], str] = _run,
) -> tuple[ReviewTag, str]:
    """Return (parsed tag, head sha) or raise ValueError naming the failure.

    `run` is injected so the checks are testable without a network or a git repo.
    Nothing here prints the API response: it carries tokens in no field today, but
    an unfiltered dump of a future schema is exactly how a secret leaks into a log.
    """
    parsed = parse_review_tag(tag)

    try:
        payload = json.loads(run(["gh", "api", f"repos/{repo}/pulls/{parsed.pr_number}"]))
    except (subprocess.CalledProcessError, json.JSONDecodeError, OSError) as exc:
        raise ValueError(
            f"could not read pull request {parsed.pr_number} in {repo}"
        ) from exc

    if payload.get("state") != "open":
        raise ValueError(f"pull request {parsed.pr_number} is not open")
    if payload.get("draft"):
        raise ValueError(f"pull request {parsed.pr_number} is a draft")

    actual_base = (payload.get("base") or {}).get("ref")
    if actual_base != base:
        raise ValueError(
            f"pull request {parsed.pr_number} targets base branch {actual_base!r}, "
            f"expected {base!r}"
        )

    head_sha = (payload.get("head") or {}).get("sha") or ""
    if not head_sha.startswith(parsed.short_sha):
        raise ValueError(
            f"tag short sha {parsed.short_sha!r} is not a prefix of the pull "
            f"request head {head_sha!r}"
        )

    try:
        target = run(["git", "rev-parse", f"{tag}^{{commit}}"]).strip()
    except (subprocess.CalledProcessError, OSError) as exc:
        raise ValueError(f"could not resolve tag {tag!r} to a commit") from exc

    if target != head_sha:
        raise ValueError(
            f"tag {tag!r} does not point at the pull request head; "
            f"tag resolves to {target!r}, head is {head_sha!r}"
        )

    return parsed, head_sha


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate_parser = sub.add_parser("validate")
    validate_parser.add_argument("--tag", required=True)
    validate_parser.add_argument("--repo", required=True)
    validate_parser.add_argument("--base", required=True)
    validate_parser.add_argument("--emit-github-output", action="store_true")
    args = parser.parse_args(argv)

    try:
        parsed, head_sha = validate(args.tag, args.repo, args.base)
    except ValueError as exc:
        print(f"review_tag: {exc}", file=sys.stderr)
        return 1

    print(f"review_tag: {args.tag} validated against PR #{parsed.pr_number}")

    if args.emit_github_output:
        output_path = os.environ.get("GITHUB_OUTPUT")
        if not output_path:
            print("review_tag: GITHUB_OUTPUT is not set", file=sys.stderr)
            return 1
        with open(output_path, "a", encoding="utf-8") as handle:
            handle.write(f"pr_number={parsed.pr_number}\n")
            handle.write(f"head_sha={head_sha}\n")
            handle.write(f"short_sha={head_sha[:12]}\n")
            handle.write(f"review_tag={args.tag}\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Verify GREEN**

Run: `uv run pytest tests/test_review_tag_template.py -v`

Expected: 14 passed.

- [ ] **Step 5: Lint and commit**

```bash
uv run ruff check skills/delivery/cloud-run-continuous-deploy/templates/review_tag.py tests/test_review_tag_template.py
uv run ruff format --check skills/delivery/cloud-run-continuous-deploy/templates/review_tag.py
git add skills/delivery/cloud-run-continuous-deploy/templates/review_tag.py tests/test_review_tag_template.py
git commit -m "feat(skills): add review tag validator template"
```

---

### Task 3: Compose inventory script

**Files:**
- Create: `skills/delivery/cloud-run-continuous-deploy/scripts/inventory-compose.sh`
- Test: `tests/test_inventory_compose.py`

**Interfaces:**
- Consumes: the skill directory from Task 1.
- Produces: `scripts/inventory-compose.sh [--compose-file PATH]`, printing a human-readable table to stdout and a starter `deploy/cloud-run.map.yml` body after a `--- worksheet ---` marker line. Task 9's SKILL.md invokes it in Phase 1.

- [ ] **Step 1: Write the failing test**

`tests/test_inventory_compose.py`:

```python
"""The inventory script must classify compose services without executing docker.

`docker compose config` is stubbed with a fake on PATH so the test pins the
classification rules, which is the part that carries judgment, rather than
docker's behavior, which does not need testing here.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

SCRIPT = (
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/scripts/inventory-compose.sh"
)

COMPOSE = {
    "services": {
        "api": {"build": {"context": "./backend"}, "ports": ["8000:8000"]},
        "web": {"build": {"context": "./frontend"}, "ports": ["3000:3000"]},
        "worker": {"build": {"context": "./backend"}, "command": "python -m app.worker"},
        "db": {"image": "postgres:16", "volumes": ["pgdata:/var/lib/postgresql/data"]},
        "cache": {"image": "redis:7"},
    }
}


@pytest.fixture
def fake_docker(tmp_path: Path) -> Path:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    (bin_dir / "docker").write_text(
        "#!/usr/bin/env bash\n"
        f"cat {json.dumps(str(tmp_path / 'compose.json'))}\n",
        encoding="utf-8",
    )
    (bin_dir / "docker").chmod(0o755)
    (tmp_path / "compose.json").write_text(json.dumps(COMPOSE), encoding="utf-8")
    return bin_dir


def _run(tmp_path: Path, bin_dir: Path) -> str:
    env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}")
    result = subprocess.run(
        ["bash", str(SCRIPT)],
        cwd=tmp_path,
        env=env,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def test_classifies_every_service(tmp_path: Path, fake_docker: Path):
    out = _run(tmp_path, fake_docker)
    assert "api" in out and "http" in out
    assert "worker" in out and "job" in out
    assert "db" in out and "externalize" in out
    assert "cache" in out and "externalize" in out


def test_names_a_managed_equivalent_for_stateful_services(tmp_path: Path, fake_docker: Path):
    out = _run(tmp_path, fake_docker)
    assert "Cloud SQL" in out
    assert "Memorystore" in out


def test_flags_named_volumes_as_stateful(tmp_path: Path, fake_docker: Path):
    out = _run(tmp_path, fake_docker)
    assert "named volume" in out


def test_emits_a_worksheet_skeleton(tmp_path: Path, fake_docker: Path):
    out = _run(tmp_path, fake_docker)
    worksheet = out.split("--- worksheet ---", 1)[1]
    assert "topology: single-container" in worksheet
    assert "compose: api" in worksheet
    assert "role: http" in worksheet
    assert "review_tag_pattern:" in worksheet


def test_fails_clearly_without_docker(tmp_path: Path):
    empty = tmp_path / "empty"
    empty.mkdir()
    env = dict(os.environ, PATH=str(empty))
    result = subprocess.run(
        ["bash", str(SCRIPT)], cwd=tmp_path, env=env, capture_output=True, text=True
    )
    assert result.returncode != 0
    assert "docker" in result.stderr
```

- [ ] **Step 2: Verify RED**

Run: `uv run pytest tests/test_inventory_compose.py -v`

Expected: every test errors — `bash: .../inventory-compose.sh: No such file or directory`.

- [ ] **Step 3: Write the script**

`skills/delivery/cloud-run-continuous-deploy/scripts/inventory-compose.sh`:

```bash
#!/usr/bin/env bash
# Read-only inventory of a docker-compose project, classified for Cloud Run.
#
# Writes nothing. Prints a table, then a starter worksheet after a marker line.
# Resolution goes through `docker compose config` so anchors, `extends` and
# interpolation are already expanded — this script never parses YAML itself.
set -euo pipefail

COMPOSE_FILE=''
while [[ $# -gt 0 ]]; do
  case "$1" in
    --compose-file)
      [[ $# -ge 2 && -n "$2" ]] || { printf 'inventory-compose: missing value for --compose-file\n' >&2; exit 2; }
      COMPOSE_FILE="$2"
      shift 2
      ;;
    *)
      printf 'inventory-compose: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

command -v docker >/dev/null 2>&1 || {
  printf 'inventory-compose: docker is required to resolve the compose file\n' >&2
  exit 2
}
command -v python3 >/dev/null 2>&1 || {
  printf 'inventory-compose: python3 is required\n' >&2
  exit 2
}

if [[ -n "$COMPOSE_FILE" ]]; then
  CONFIG="$(docker compose --file "$COMPOSE_FILE" config --format json)"
else
  CONFIG="$(docker compose config --format json)"
fi

printf '%s' "$CONFIG" | python3 - <<'PY'
import json
import sys

STATEFUL = {
    "postgres": "Cloud SQL, Supabase or AlloyDB",
    "mysql": "Cloud SQL",
    "mariadb": "Cloud SQL",
    "redis": "Memorystore",
    "valkey": "Memorystore",
    "memcached": "Memorystore",
    "minio": "Cloud Storage",
    "elasticsearch": "Vertex AI Search or self-hosted GCE",
    "opensearch": "Vertex AI Search or self-hosted GCE",
    "rabbitmq": "Pub/Sub",
    "kafka": "Pub/Sub",
    "clickhouse": "BigQuery or self-hosted GCE",
    "neo4j": "self-hosted GCE",
    "qdrant": "Vertex AI Vector Search or self-hosted GCE",
    "weaviate": "Vertex AI Vector Search or self-hosted GCE",
    "chroma": "Vertex AI Vector Search or self-hosted GCE",
}


def image_family(image):
    # "docker.io/library/postgres:16" -> "postgres"
    return image.split("/")[-1].split(":")[0].lower()


def first_port(service):
    for entry in service.get("ports") or []:
        target = entry.get("target") if isinstance(entry, dict) else None
        if target:
            return int(target)
        if isinstance(entry, str) and ":" in entry:
            return int(entry.rsplit(":", 1)[1].split("/")[0])
    for entry in service.get("expose") or []:
        return int(str(entry).split("/")[0])
    return None


def named_volumes(service):
    names = []
    for entry in service.get("volumes") or []:
        source = entry.get("source") if isinstance(entry, dict) else str(entry).split(":")[0]
        kind = entry.get("type") if isinstance(entry, dict) else None
        if source and not source.startswith((".", "/")) and kind != "bind":
            names.append(source)
    return names


config = json.load(sys.stdin)
services = config.get("services") or {}

rows = []
for name, service in sorted(services.items()):
    image = service.get("image") or ""
    family = image_family(image) if image else ""
    port = first_port(service)
    volumes = named_volumes(service)
    managed = STATEFUL.get(family, "")

    if managed:
        role, note = "externalize", managed
    elif volumes:
        role, note = "externalize", f"named volume {volumes[0]!r}; Cloud Run has no persistent disk"
    elif not service.get("build"):
        role, note = "externalize", "no build context; nothing to deploy from this repo"
    elif port:
        role, note = "http", f"port {port}"
    else:
        role, note = "job", "no published port"

    rows.append((name, role, port, note, service))

width = max((len(r[0]) for r in rows), default=7)
print(f"{'service'.ljust(width)}  role         notes")
print(f"{'-' * width}  -----------  -----")
for name, role, _, note, _ in rows:
    print(f"{name.ljust(width)}  {role.ljust(11)}  {note}")

http = [r for r in rows if r[1] == "http"]
print()
print(f"{len(rows)} service(s): {len(http)} http, "
      f"{len([r for r in rows if r[1] == 'job'])} job, "
      f"{len([r for r in rows if r[1] == 'externalize'])} to externalize.")
if len(http) > 1:
    print("More than one http service: single-container topology is available. "
          "Confirm path prefixes with the operator before generating.")

print()
print("--- worksheet ---")
print("# Starter deploy/cloud-run.map.yml. Every value below is a guess except the")
print("# service names. Confirm each one with the operator before generating.")
print("project_id: CHANGEME")
print("region: us-central1")
print("ar_repo: CHANGEME")
print("topology: single-container")
print("branches:")
print("  dev: dev")
print("  prod: main")
print('review_tag_pattern: "review/pr-*/*"')
print("services:")
for name, role, port, _, service in rows:
    print(f"  - compose: {name}")
    print(f"    role: {role}")
    if role == "http":
        print(f"    port: {port}")
        print(f"    path_prefix: /{name}")
    elif role == "job":
        command = service.get("command")
        if isinstance(command, str):
            command = command.split()
        print(f"    command: {json.dumps(command or [])}")
    else:
        family = image_family(service.get("image") or "")
        print(f"    managed: {STATEFUL.get(family, 'CHANGEME')}")
PY
```

Note the `CHANGEME` markers are output of a generated worksheet, not plan placeholders — they exist so an unreviewed worksheet cannot be mistaken for a confirmed one.

- [ ] **Step 4: Verify GREEN**

Run: `uv run pytest tests/test_inventory_compose.py -v`

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
chmod +x skills/delivery/cloud-run-continuous-deploy/scripts/inventory-compose.sh
bash -n skills/delivery/cloud-run-continuous-deploy/scripts/inventory-compose.sh
git add skills/delivery/cloud-run-continuous-deploy/scripts/inventory-compose.sh tests/test_inventory_compose.py
git commit -m "feat(skills): add compose inventory for cloud run mapping"
```

---

### Task 4: Varlock to GitHub Environment secret sync

**Files:**
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/sync-github-secrets.sh`
- Test: `tests/test_sync_github_secrets.py`

**Interfaces:**
- Consumes: the skill directory from Task 1.
- Produces: `scripts/sync-github-secrets.sh <dev|prod|integration>`, invoked as `APP_ENV=<env> varlock run --inject vars -- scripts/sync-github-secrets.sh <env>`. Task 9's SKILL.md Phase 2 uses exactly that invocation.

- [ ] **Step 1: Write the failing test**

`tests/test_sync_github_secrets.py`:

```python
"""The sync script is the one place a secret value and the agent nearly meet.

Its tests therefore pin two things: the environment-suffix rule that stops a
missing integration secret falling back to a repository-scoped one, and the
absolute silence of the script about values.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

SCRIPT = (
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/templates/sync-github-secrets.sh"
)

SENSITIVE = {"config": {"DATABASE_URL": {"value": "REDACTED"}, "API_KEY": {"value": "REDACTED"}}}


@pytest.fixture
def fake_bin(tmp_path: Path) -> tuple[Path, Path]:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    calls = tmp_path / "gh-calls.txt"

    (bin_dir / "gh").write_text(
        "#!/usr/bin/env bash\n"
        'if [[ "$1" == "auth" ]]; then exit 0; fi\n'
        f'printf "%s\\n" "$*" >> {json.dumps(str(calls))}\n'
        "cat > /dev/null\n",
        encoding="utf-8",
    )
    (bin_dir / "varlock").write_text(
        "#!/usr/bin/env bash\n" f"printf '%s' {json.dumps(json.dumps(SENSITIVE))}\n",
        encoding="utf-8",
    )
    for name in ("gh", "varlock"):
        (bin_dir / name).chmod(0o755)
    return bin_dir, calls


def _run(bin_dir: Path, environment: str, values: dict[str, str]):
    env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}", **values)
    return subprocess.run(
        ["bash", str(SCRIPT), environment], env=env, capture_output=True, text=True
    )


def test_dev_uses_bare_secret_names(fake_bin):
    bin_dir, calls = fake_bin
    result = _run(bin_dir, "dev", {"DATABASE_URL": "postgres://x", "API_KEY": "k"})
    assert result.returncode == 0, result.stderr
    recorded = calls.read_text(encoding="utf-8")
    assert "secret set DATABASE_URL --env dev" in recorded
    assert "DATABASE_URL_DEV" not in recorded


def test_integration_suffixes_every_secret_name(fake_bin):
    bin_dir, calls = fake_bin
    result = _run(bin_dir, "integration", {"DATABASE_URL": "postgres://x", "API_KEY": "k"})
    assert result.returncode == 0, result.stderr
    recorded = calls.read_text(encoding="utf-8")
    assert "secret set DATABASE_URL_INTEGRATION --env integration" in recorded
    assert "secret set API_KEY_INTEGRATION --env integration" in recorded


def test_never_prints_a_value(fake_bin):
    bin_dir, _ = fake_bin
    result = _run(bin_dir, "dev", {"DATABASE_URL": "postgres://supersecret", "API_KEY": "k"})
    assert "supersecret" not in result.stdout
    assert "supersecret" not in result.stderr


def test_never_passes_a_value_in_argv(fake_bin):
    bin_dir, calls = fake_bin
    _run(bin_dir, "dev", {"DATABASE_URL": "postgres://supersecret", "API_KEY": "k"})
    assert "supersecret" not in calls.read_text(encoding="utf-8")


def test_fails_when_a_declared_secret_is_unset(fake_bin):
    bin_dir, _ = fake_bin
    result = _run(bin_dir, "integration", {"DATABASE_URL": "postgres://x"})
    assert result.returncode != 0
    assert "API_KEY" in result.stderr


def test_rejects_an_unknown_environment(fake_bin):
    bin_dir, _ = fake_bin
    result = _run(bin_dir, "staging", {"DATABASE_URL": "x", "API_KEY": "k"})
    assert result.returncode != 0
    assert "staging" in result.stderr
```

- [ ] **Step 2: Verify RED**

Run: `uv run pytest tests/test_sync_github_secrets.py -v`

Expected: all six fail — the script does not exist, so bash exits 127.

- [ ] **Step 3: Write the template**

`skills/delivery/cloud-run-continuous-deploy/templates/sync-github-secrets.sh`:

```bash
#!/usr/bin/env bash
# Sync every @sensitive Varlock item into one GitHub Environment.
#
# Always invoke through varlock, so values arrive as process environment
# variables and never pass through an agent, a file, or a command line:
#
#   APP_ENV=integration varlock run --inject vars -- \
#     scripts/sync-github-secrets.sh integration
#
# bash, not sh: ${!name} indirect expansion is a bashism.
set -euo pipefail

die() {
  printf 'sync-github-secrets: %s\n' "$1" >&2
  exit 2
}

ENVIRONMENT="${1:-}"
[[ -n "$ENVIRONMENT" ]] || die 'usage: sync-github-secrets.sh <dev|prod|integration>'
case "$ENVIRONMENT" in
  dev|prod|integration) ;;
  *) die "unknown environment: $ENVIRONMENT (expected dev, prod or integration)" ;;
esac

command -v gh >/dev/null 2>&1 || die 'gh CLI is required'
command -v varlock >/dev/null 2>&1 || die 'varlock is required'
command -v python3 >/dev/null 2>&1 || die 'python3 is required'
gh auth status >/dev/null 2>&1 || die 'gh is not authenticated; run: gh auth login'

# Names only. --agent redacts values, and neither flag is --format, so this stays
# outside the agent deny-glob in opencode.jsonc and no value can reach a
# transcript. .env.schema therefore remains the only list of what exists.
KEYS="$(
  varlock load --agent --filter='@sensitive' |
    python3 -c 'import json,sys; d=json.load(sys.stdin); print("\n".join((d.get("config") or d).keys()))'
)"
[[ -n "$KEYS" ]] || die 'no @sensitive items declared in .env.schema'

# The destructive environment gets suffixed names. GitHub resolves a secret
# reference environment -> repository -> organization, so a bare DATABASE_URL
# missing from the integration Environment would silently resolve to a
# repository-scoped one -- and that DSN reaches a job whose purpose is to drop a
# schema. Suffixed, a missing secret resolves empty and the workflow fails loudly.
target_name() {
  if [[ "$ENVIRONMENT" == integration ]]; then
    printf '%s_INTEGRATION' "$1"
  else
    printf '%s' "$1"
  fi
}

count=0
while IFS= read -r key; do
  [[ -n "$key" ]] || continue
  value="${!key-}"
  [[ -n "$value" ]] || die "$key is unset or empty for environment $ENVIRONMENT"
  target="$(target_name "$key")"
  # stdin, never --body: an argv value is visible in ps.
  printf '%s' "$value" | gh secret set "$target" --env "$ENVIRONMENT" >/dev/null
  printf '  %s -> %s ok\n' "$key" "$target"
  count=$((count + 1))
done <<<"$KEYS"

printf '%d secret(s) synced to the %s environment; 0 values printed.\n' "$count" "$ENVIRONMENT"
```

- [ ] **Step 4: Verify GREEN**

Run: `uv run pytest tests/test_sync_github_secrets.py -v`

Expected: 6 passed.

- [ ] **Step 5: Commit**

```bash
bash -n skills/delivery/cloud-run-continuous-deploy/templates/sync-github-secrets.sh
git add skills/delivery/cloud-run-continuous-deploy/templates/sync-github-secrets.sh tests/test_sync_github_secrets.py
git commit -m "feat(skills): add varlock to github environment secret sync"
```

---

### Task 5: GCP bootstrap template

**Files:**
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/bootstrap-gcp.sh`
- Test: `tests/test_cloud_run_deploy_skill.py`

**Interfaces:**
- Consumes: the skill directory from Task 1.
- Produces: `scripts/bootstrap-gcp.sh --project PROJECT [--region REGION] [--integration-region REGION] --repo ORG/REPO`, printing the WIF provider resource name. Task 9's SKILL.md Phase 3 hands it to the operator. Also produces the shared test module `tests/test_cloud_run_deploy_skill.py` with `TEMPLATES` and `_template(name)`, which Tasks 6, 7 and 8 extend.

- [ ] **Step 1: Write the failing invariant tests**

`tests/test_cloud_run_deploy_skill.py`:

```python
"""Static invariant tests over the shipped templates.

These exist because the skill's value is a dozen security invariants, and prose
does not fail CI. A template edit that drops one fails here instead of in a
production deploy. Every assertion names the invariant from the spec.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

TEMPLATES = (
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/templates"
)


def _template(name: str) -> str:
    return (TEMPLATES / name).read_text(encoding="utf-8")


ALL_TEMPLATE_NAMES = [
    "bootstrap-gcp.sh",
    "deploy.yml",
    "deploy-integration.yml",
    "integration-tag.yml",
    "review_tag.py",
    "sync-github-secrets.sh",
]


@pytest.mark.parametrize("name", ALL_TEMPLATE_NAMES)
def test_no_template_creates_a_service_account_key(name: str):
    # Invariant 12: WIF only. A key file in a repo or on a laptop is a permanent,
    # unrevoked credential the moment it leaks.
    assert "service-accounts keys create" not in _template(name)


def test_wif_condition_pins_repository_and_branch_refs():
    # Invariant 4: widening the condition to tag refs hands a federated token to
    # anyone who can push a tag.
    script = _template("bootstrap-gcp.sh")
    assert "assertion.repository ==" in script
    assert "refs/heads/" in script
    assert "refs/tags/" not in script


def test_bootstrap_fails_closed_on_ambiguous_inspection():
    # Invariant 6: treating an ambiguous describe error as "absent" is how a
    # bootstrap recreates or resets a live resource.
    script = _template("bootstrap-gcp.sh")
    assert "refusing to treat it as absent" in script


def test_bootstrap_grants_no_project_level_role_to_a_runtime_account():
    # Invariant 8: runtime identities get resource-scoped bindings only.
    script = _template("bootstrap-gcp.sh")
    for line in script.splitlines():
        if "projects add-iam-policy-binding" in line:
            assert "RUNTIME" not in line, line
```

- [ ] **Step 2: Verify RED**

Run: `uv run pytest tests/test_cloud_run_deploy_skill.py -v`

Expected: every test errors with `FileNotFoundError` — no template exists yet. The parametrized cases for templates from later tasks will also fail; that is expected and they go green as Tasks 6–8 land.

- [ ] **Step 3: Write the bootstrap template**

`skills/delivery/cloud-run-continuous-deploy/templates/bootstrap-gcp.sh`. Substitute `{{AR_REPO}}` throughout when generating.

```bash
#!/usr/bin/env bash
# Idempotent bootstrap of the GCP resources this repo's deployment needs.
#
#   scripts/bootstrap-gcp.sh --project PROJECT [--region REGION] \
#     [--integration-region REGION] --repo ORG/REPO
#
# Run by an operator with project-admin rights. The deploy identity this script
# creates must never hold them.
set -euo pipefail

usage() {
  printf 'Usage: %s --project PROJECT [--region REGION] [--integration-region REGION] --repo ORG/REPO\n' "$0" >&2
}

die() {
  printf 'bootstrap-gcp: %s\n' "$1" >&2
  usage
  exit 2
}

PROJECT=''
REGION='us-central1'
INTEGRATION_REGION=''
REPO=''

while [[ $# -gt 0 ]]; do
  case "$1" in
    --project|--region|--integration-region|--repo)
      [[ $# -ge 2 && -n "$2" && "$2" != -* ]] || die "missing value for $1"
      case "$1" in
        --project) [[ -z "$PROJECT" ]] || die 'duplicate --project'; PROJECT="$2" ;;
        --region) REGION="$2" ;;
        --integration-region) INTEGRATION_REGION="$2" ;;
        --repo) [[ -z "$REPO" ]] || die 'duplicate --repo'; REPO="$2" ;;
      esac
      shift 2
      ;;
    *) die "unknown argument: $1" ;;
  esac
done

[[ -n "$PROJECT" ]] || die 'missing required --project'
[[ -n "$REPO" ]] || die 'missing required --repo'
[[ "$REPO" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || die '--repo must be ORG/REPO'
[[ "$PROJECT" =~ ^[a-z0-9-]+$ ]] || die '--project must be lowercase letters, digits and hyphens'
[[ -n "$INTEGRATION_REGION" ]] || INTEGRATION_REGION="$REGION"

command -v gcloud >/dev/null 2>&1 || die 'gcloud CLI is required'

AR_REPO='{{AR_REPO}}'
DEV_BRANCH='{{DEV_BRANCH}}'
PROD_BRANCH='{{PROD_BRANCH}}'
POOL='github-pool'
PROVIDER='github-provider'
DEPLOY_SA="${AR_REPO}-deployer"
DEPLOY_SA_EMAIL="${DEPLOY_SA}@${PROJECT}.iam.gserviceaccount.com"

lowercase() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }

DESCRIBE_OUTPUT=''

# Fail closed. A describe that fails for any reason other than a recognised
# not-found signature aborts: treating an ambiguous error as "absent" is how a
# bootstrap silently recreates or resets a live resource.
describe_resource() {
  local output
  if output=$("$@" 2>&1); then
    DESCRIBE_OUTPUT="$output"
    return 0
  fi
  shopt -s nocasematch
  case "$output" in
    *'not_found'*|*'not found'*|*'does not exist'*|*'http 404'*|*'404 not found'*)
      shopt -u nocasematch
      DESCRIBE_OUTPUT="$output"
      return 1
      ;;
  esac
  shopt -u nocasematch
  printf 'bootstrap-gcp: resource inspection failed; refusing to treat it as absent:\n%s\n' \
    "${output:-<no diagnostic output>}" >&2
  exit 1
}

printf '== Enabling APIs ==\n'
gcloud services enable \
  artifactregistry.googleapis.com cloudresourcemanager.googleapis.com \
  iam.googleapis.com iamcredentials.googleapis.com run.googleapis.com \
  secretmanager.googleapis.com storage.googleapis.com sts.googleapis.com \
  --project="$PROJECT"

PROJECT_NUM="$(gcloud projects describe "$PROJECT" --format='value(projectNumber)')"

printf '== Artifact Registry ==\n'
if describe_resource gcloud artifacts repositories describe "$AR_REPO" \
  --location="$REGION" --project="$PROJECT" --format='value(format,location)'; then
  read -r repo_format repo_location <<<"$DESCRIBE_OUTPUT"
  [[ "$(lowercase "$repo_format")" == 'docker' ]] || die "$AR_REPO is not a Docker repository"
  [[ "$(lowercase "$repo_location")" == "$(lowercase "$REGION")" ]] ||
    die "$AR_REPO is in $repo_location, expected $REGION"
else
  gcloud artifacts repositories create "$AR_REPO" \
    --repository-format=docker --location="$REGION" --project="$PROJECT"
fi

printf '== Workload Identity Federation ==\n'
# Branch refs only. Widening this to tag refs would hand a federated token to
# anyone who can push a tag, which is the whole reason the review-tag path runs
# its privileged half from the trusted branch instead.
ATTRIBUTE_CONDITION="assertion.repository == '${REPO}' && (assertion.ref == 'refs/heads/${DEV_BRANCH}' || assertion.ref == 'refs/heads/${PROD_BRANCH}')"

if ! describe_resource gcloud iam workload-identity-pools describe "$POOL" \
  --location=global --project="$PROJECT"; then
  gcloud iam workload-identity-pools create "$POOL" --location=global --project="$PROJECT"
fi

if describe_resource gcloud iam workload-identity-pools providers describe "$PROVIDER" \
  --location=global --workload-identity-pool="$POOL" --project="$PROJECT"; then
  PROVIDER_VERB='update-oidc'
else
  PROVIDER_VERB='create-oidc'
fi

gcloud iam workload-identity-pools providers "$PROVIDER_VERB" "$PROVIDER" \
  --location=global \
  --workload-identity-pool="$POOL" \
  --issuer-uri=https://token.actions.githubusercontent.com \
  --attribute-mapping='google.subject=assertion.sub,attribute.repository=assertion.repository,attribute.ref=assertion.ref' \
  --attribute-condition="$ATTRIBUTE_CONDITION" \
  --project="$PROJECT"

printf '== Service accounts ==\n'
RUNTIME_ACCOUNTS=()
for environment in dev prod integration; do
  RUNTIME_ACCOUNTS+=("${AR_REPO}-runtime-${environment}")
done

for account in "$DEPLOY_SA" "${RUNTIME_ACCOUNTS[@]}"; do
  email="${account}@${PROJECT}.iam.gserviceaccount.com"
  if ! describe_resource gcloud iam service-accounts describe "$email" --project="$PROJECT"; then
    gcloud iam service-accounts create "$account" \
      --display-name="${AR_REPO} deploy (${account})" --project="$PROJECT"
  fi
done

printf '== Deployer roles ==\n'
# run.admin is the only project-level grant, and only the deployer holds it.
gcloud projects add-iam-policy-binding "$PROJECT" \
  --member="serviceAccount:${DEPLOY_SA_EMAIL}" \
  --role=roles/run.admin --condition=None >/dev/null

gcloud artifacts repositories add-iam-policy-binding "$AR_REPO" \
  --location="$REGION" --project="$PROJECT" \
  --member="serviceAccount:${DEPLOY_SA_EMAIL}" \
  --role=roles/artifactregistry.writer >/dev/null

for account in "${RUNTIME_ACCOUNTS[@]}"; do
  gcloud iam service-accounts add-iam-policy-binding \
    "${account}@${PROJECT}.iam.gserviceaccount.com" \
    --project="$PROJECT" \
    --member="serviceAccount:${DEPLOY_SA_EMAIL}" \
    --role=roles/iam.serviceAccountUser >/dev/null
done

printf '== WIF binding: repository -> deployer ==\n'
gcloud iam service-accounts add-iam-policy-binding "$DEPLOY_SA_EMAIL" \
  --project="$PROJECT" --role=roles/iam.workloadIdentityUser \
  --member="principalSet://iam.googleapis.com/projects/${PROJECT_NUM}/locations/global/workloadIdentityPools/${POOL}/attribute.repository/${REPO}" >/dev/null

printf '== Secret Manager ==\n'
# Placeholders only. Real values arrive from GitHub Environments at deploy time,
# so no value passes through this script or an operator terminal.
SECRET_KEYS=({{SECRET_KEYS}})

for environment in dev prod integration; do
  runtime_email="${AR_REPO}-runtime-${environment}@${PROJECT}.iam.gserviceaccount.com"
  for key in "${SECRET_KEYS[@]}"; do
    secret="${key}-${environment}"
    if ! describe_resource gcloud secrets describe "$secret" --project="$PROJECT"; then
      printf '%s' 'placeholder-set-by-deploy-workflow' |
        gcloud secrets create "$secret" --replication-policy=automatic \
          --data-file=- --project="$PROJECT"
    fi
    gcloud secrets add-iam-policy-binding "$secret" --project="$PROJECT" \
      --member="serviceAccount:${DEPLOY_SA_EMAIL}" \
      --role=roles/secretmanager.secretVersionAdder >/dev/null
    gcloud secrets add-iam-policy-binding "$secret" --project="$PROJECT" \
      --member="serviceAccount:${runtime_email}" \
      --role=roles/secretmanager.secretAccessor >/dev/null
  done
done

printf '== Uploads buckets ==\n'
for environment in dev prod integration; do
  bucket="${AR_REPO}-${environment}-uploads"
  runtime_email="${AR_REPO}-runtime-${environment}@${PROJECT}.iam.gserviceaccount.com"
  bucket_region="$REGION"
  [[ "$environment" == integration ]] && bucket_region="$INTEGRATION_REGION"

  if describe_resource gcloud storage buckets describe "gs://${bucket}" \
    --project="$PROJECT" --raw \
    --format='value(projectNumber,location,iamConfiguration.uniformBucketLevelAccess.enabled)'; then
    read -r bucket_project bucket_location bucket_uniform <<<"$DESCRIBE_OUTPUT"
    [[ "$bucket_project" == "$PROJECT_NUM" ]] ||
      die "bucket $bucket belongs to project number $bucket_project, expected $PROJECT_NUM"
    [[ "$(lowercase "$bucket_location")" == "$(lowercase "$bucket_region")" ]] ||
      die "bucket $bucket is in $bucket_location, expected $bucket_region"
    [[ "$(lowercase "$bucket_uniform")" == 'true' ]] ||
      die "bucket $bucket does not have uniform bucket-level access"
  else
    gcloud storage buckets create "gs://${bucket}" \
      --location="$bucket_region" --uniform-bucket-level-access --project="$PROJECT"
  fi

  gcloud storage buckets add-iam-policy-binding "gs://${bucket}" \
    --project="$PROJECT" \
    --member="serviceAccount:${runtime_email}" \
    --role=roles/storage.objectAdmin >/dev/null
done

WIF_PROVIDER="projects/${PROJECT_NUM}/locations/global/workloadIdentityPools/${POOL}/providers/${PROVIDER}"

printf '\nBootstrap complete.\n\n'
printf 'Set these repository secrets (Settings -> Secrets and variables -> Actions):\n'
printf '  GCP_PROJECT_ID = %s\n' "$PROJECT"
printf '  WIF_PROVIDER   = %s\n\n' "$WIF_PROVIDER"
printf 'Runtime service accounts:\n'
for account in "${RUNTIME_ACCOUNTS[@]}"; do
  printf '  %s@%s.iam.gserviceaccount.com\n' "$account" "$PROJECT"
done
printf '\nAuthentication is WIF only. Do not create, download or store service-account keys.\n'
```

- [ ] **Step 4: Verify the bootstrap tests pass**

```bash
uv run pytest tests/test_cloud_run_deploy_skill.py -v -k "wif or fails_closed or project_level or bootstrap"
bash -n skills/delivery/cloud-run-continuous-deploy/templates/bootstrap-gcp.sh
```

Expected: the four bootstrap-specific tests pass. The parametrized `test_no_template_creates_a_service_account_key` cases for `deploy.yml`, `deploy-integration.yml` and `integration-tag.yml` still fail — those templates land in Tasks 6 and 7.

- [ ] **Step 5: Commit**

```bash
git add skills/delivery/cloud-run-continuous-deploy/templates/bootstrap-gcp.sh tests/test_cloud_run_deploy_skill.py
git commit -m "feat(skills): add gcp bootstrap template with fail-closed inspection"
```

---

### Task 6: The review-tag trust boundary

The listener and the trusted workflow are one deliverable. Splitting them would let a reviewer approve a listener whose counterpart never re-validates — which is exactly the bug the pair exists to prevent.

**Files:**
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/integration-tag.yml`
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/deploy-integration.yml`
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/integration-marker.sql`
- Modify: `tests/test_cloud_run_deploy_skill.py`

**Interfaces:**
- Consumes: `review_tag.py`'s CLI from Task 2; `_template()` from Task 5.
- Produces: workflow `deploy-integration.yml` accepting one `workflow_dispatch` input `review_tag`, with jobs `validate` (outputs `pr_number`, `head_sha`, `short_sha`, `review_tag`) and `deploy`.

- [ ] **Step 1: Add the failing trust-boundary tests**

Append to `tests/test_cloud_run_deploy_skill.py`:

```python
def test_listener_never_receives_credentials():
    # Invariants 1 and 2: this file is loaded from the tagged commit's own tree,
    # so its author controls every line. It must be worth nothing to compromise.
    listener = _template("integration-tag.yml")
    assert "google-github-actions/auth" not in listener
    assert "${{ secrets." not in listener
    assert "environment:" not in listener


def test_listener_documents_that_it_is_not_a_boundary():
    # The comment is load-bearing: without it the next reader adds a check here
    # and believes it protects something.
    listener = _template("integration-tag.yml")
    assert "not a security control" in listener.lower() or "never a boundary" in listener.lower()


def test_validate_job_runs_before_any_credential():
    # Invariant 1: ordering is the boundary. deploy must depend on validate.
    workflow = _template("deploy-integration.yml")
    assert re.search(r"needs:\s*validate", workflow)
    validate_block = workflow.split("deploy:", 1)[0]
    assert "google-github-actions/auth" not in validate_block
    assert "environment:" not in validate_block


def test_untrusted_head_checkout_drops_credentials():
    # Invariant 5: a persisted token in a checkout of attacker-controlled code is
    # a token in attacker-controlled code.
    workflow = _template("deploy-integration.yml")
    assert "persist-credentials: false" in workflow


def test_integration_concurrency_does_not_cancel():
    # Invariant 11: cancelling mid-reset leaves a half-migrated database.
    workflow = _template("deploy-integration.yml")
    assert "cancel-in-progress: false" in workflow


def test_reset_requires_both_flags():
    # Invariant 10, the two flags. The marker is asserted separately.
    workflow = _template("deploy-integration.yml")
    assert "ALLOW_DATABASE_RESET=true" in workflow
    assert "DEPLOY_ENV=integration" in workflow


def test_marker_lives_outside_the_dropped_schema():
    # Invariant 10, the marker. A marker inside `public` is destroyed by the very
    # reset it is supposed to authorise, so the second run would find none.
    marker = _template("integration-marker.sql")
    assert "review_control" in marker
    assert "public." not in marker


def test_integration_images_are_tagged_by_pr_and_sha():
    # Invariant 9: a deployed revision must be traceable to one commit.
    workflow = _template("deploy-integration.yml")
    assert "integration-pr-" in workflow
    assert ":latest" not in workflow
```

- [ ] **Step 2: Verify RED**

Run: `uv run pytest tests/test_cloud_run_deploy_skill.py -v -k "listener or validate_job or untrusted or concurrency or reset or marker or integration_images"`

Expected: all eight fail with `FileNotFoundError`.

- [ ] **Step 3: Write the listener**

`skills/delivery/cloud-run-continuous-deploy/templates/integration-tag.yml`:

```yaml
name: Integration review tag

on:
  push:
    tags: ["{{REVIEW_TAG_PATTERN}}"]

# This workflow is loaded from the TAGGED COMMIT'S OWN TREE. Whoever creates the
# tag controls every line of this file, so nothing here is a security control --
# it is fail-fast ergonomics that keeps the trusted workflow's queue clean.
# The real boundaries are the WIF attribute condition, which accepts branch refs
# only, and the re-validation inside deploy-integration.yml. Both must hold with
# this file assumed hostile.
permissions:
  contents: read
  actions: write
  pull-requests: read

env:
  GH_TOKEN: ${{ github.token }}

jobs:
  dispatch:
    runs-on: ubuntu-latest
    steps:
      - name: Check out trusted control code
        uses: actions/checkout@v4
        with:
          ref: {{DEV_BRANCH}}
          path: control
          fetch-depth: 1

      - name: Validate review tag
        working-directory: control
        run: |
          set -euo pipefail
          git fetch --force --no-tags origin \
            "refs/tags/$GITHUB_REF_NAME:refs/tags/$GITHUB_REF_NAME"
          python3 scripts/review_tag.py validate \
            --tag "$GITHUB_REF_NAME" \
            --repo "$GITHUB_REPOSITORY" \
            --base {{DEV_BRANCH}}

      - name: Dispatch trusted integration deployment
        run: |
          set -euo pipefail
          gh workflow run deploy-integration.yml \
            --repo "$GITHUB_REPOSITORY" \
            --ref {{DEV_BRANCH}} \
            -f "review_tag=$GITHUB_REF_NAME"
```

- [ ] **Step 4: Write the trusted deployment workflow**

`skills/delivery/cloud-run-continuous-deploy/templates/deploy-integration.yml`. This is the single-container form; for `per-service`, repeat the build/deploy steps per service and add the CORS step from `deploy.yml`.

```yaml
name: Deploy integration

# Dispatched by integration-tag.yml, which is untrusted. Everything this
# workflow believes about the tag it establishes itself, from the trusted
# branch, before any credential exists in the job.
on:
  workflow_dispatch:
    inputs:
      review_tag:
        description: "Immutable review tag (review/pr-<number>/<short-sha>)"
        type: string
        required: true

env:
  AR_REGION: {{REGION}}
  REGION: {{INTEGRATION_REGION}}
  AR_REPO: {{AR_REPO}}
  SERVICE: {{AR_REPO}}-integration
  RUNTIME_SA: {{AR_REPO}}-runtime-integration

jobs:
  validate:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
    env:
      GH_TOKEN: ${{ github.token }}
    outputs:
      pr_number: ${{ steps.review.outputs.pr_number }}
      head_sha: ${{ steps.review.outputs.head_sha }}
      short_sha: ${{ steps.review.outputs.short_sha }}
      review_tag: ${{ steps.review.outputs.review_tag }}
    steps:
      - name: Check out trusted control code
        uses: actions/checkout@v4
        with:
          ref: {{DEV_BRANCH}}
          path: control
          fetch-depth: 1

      - name: Validate review tag against its pull request
        id: review
        working-directory: control
        env:
          REVIEW_TAG: ${{ inputs.review_tag }}
        run: |
          set -euo pipefail
          git fetch --force --no-tags origin "refs/tags/${REVIEW_TAG}:refs/tags/${REVIEW_TAG}"
          python3 scripts/review_tag.py validate \
            --tag "$REVIEW_TAG" \
            --repo "$GITHUB_REPOSITORY" \
            --base {{DEV_BRANCH}} \
            --emit-github-output

  deploy:
    needs: validate
    runs-on: ubuntu-latest
    environment: integration
    permissions:
      id-token: write
      contents: read
    concurrency:
      group: integration
      cancel-in-progress: false
    timeout-minutes: 45
    env:
      GCP_PROJECT_ID: ${{ secrets.GCP_PROJECT_ID }}
      PR_NUMBER: ${{ needs.validate.outputs.pr_number }}
      HEAD_SHA: ${{ needs.validate.outputs.head_sha }}
      SHORT_SHA: ${{ needs.validate.outputs.short_sha }}
    steps:
      - name: Validate deployment target
        run: |
          set -euo pipefail
          [[ "$GCP_PROJECT_ID" =~ ^[a-z0-9-]+$ ]] ||
            { echo "GCP_PROJECT_ID is malformed" >&2; exit 1; }
          [[ "$HEAD_SHA" =~ ^[0-9a-f]{40}$ ]] ||
            { echo "head sha was not validated" >&2; exit 1; }

      - name: Check out the validated pull request head
        uses: actions/checkout@v4
        with:
          ref: ${{ needs.validate.outputs.head_sha }}
          path: source
          fetch-depth: 1
          # This tree is attacker-controlled. A persisted token here is a token
          # in attacker-controlled code.
          persist-credentials: false

      - name: Authenticate to Google Cloud
        id: auth
        uses: google-github-actions/auth@v2
        with:
          token_format: access_token
          create_credentials_file: true
          project_id: ${{ secrets.GCP_PROJECT_ID }}
          workload_identity_provider: ${{ secrets.WIF_PROVIDER }}
          service_account: {{AR_REPO}}-deployer@${{ secrets.GCP_PROJECT_ID }}.iam.gserviceaccount.com

      - name: Set up gcloud
        uses: google-github-actions/setup-gcloud@v2
        with:
          project_id: ${{ secrets.GCP_PROJECT_ID }}

      - name: Log in to Artifact Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.AR_REGION }}-docker.pkg.dev
          username: oauth2accesstoken
          password: ${{ steps.auth.outputs.access_token }}

      - name: Build and push the image
        working-directory: source
        run: |
          set -euo pipefail
          IMAGE="${AR_REGION}-docker.pkg.dev/${GCP_PROJECT_ID}/${AR_REPO}/app:integration-pr-${PR_NUMBER}-${SHORT_SHA}"
          docker build -f deploy/Dockerfile -t "$IMAGE" .
          docker push "$IMAGE"
          echo "IMAGE=$IMAGE" >> "$GITHUB_ENV"

      - name: Sync integration secrets to Secret Manager
        env:
          {{SECRET_ENV_BLOCK}}
        run: |
          set -euo pipefail
          sync_secret() {
            local name="$1" value="$2"
            if [[ -z "$value" ]]; then
              echo "refusing to sync empty value into ${name}; the integration" \
                   "Environment secret is missing or unset" >&2
              exit 1
            fi
            printf '%s' "$value" | gcloud secrets versions add "$name" \
              --data-file=- --project "$GCP_PROJECT_ID"
          }
          {{SYNC_SECRET_CALLS}}

      - name: Reset, migrate and seed the integration database
        run: |
          set -euo pipefail
          gcloud run jobs deploy "reset-integration" \
            --image "$IMAGE" \
            --region "$REGION" \
            --project "$GCP_PROJECT_ID" \
            --service-account "${RUNTIME_SA}@${GCP_PROJECT_ID}.iam.gserviceaccount.com" \
            --command {{RESET_ENTRYPOINT}} \
            --max-retries 0 \
            --set-env-vars "DEPLOY_ENV=integration,ALLOW_DATABASE_RESET=true" \
            --set-secrets "{{RESET_SECRETS}}"
          gcloud run jobs execute "reset-integration" \
            --region "$REGION" --project "$GCP_PROJECT_ID" --wait

      - name: Deploy the integration service
        id: service
        run: |
          set -euo pipefail
          gcloud run deploy "$SERVICE" \
            --image "$IMAGE" \
            --region "$REGION" \
            --project "$GCP_PROJECT_ID" \
            --allow-unauthenticated \
            --service-account "${RUNTIME_SA}@${GCP_PROJECT_ID}.iam.gserviceaccount.com" \
            --cpu 1 --memory 1Gi --min 0 --max 1 \
            --set-env-vars "DEPLOY_ENV=integration,EXPOSE_API_DOCS=true" \
            --set-secrets "{{RUNTIME_SECRETS}}"
          URL="$(gcloud run services describe "$SERVICE" \
            --region "$REGION" --project "$GCP_PROJECT_ID" --format 'value(status.url)')"
          echo "url=$URL" >> "$GITHUB_OUTPUT"

      - name: Smoke test
        env:
          URL: ${{ steps.service.outputs.url }}
        run: |
          set -euo pipefail
          curl --fail --silent --show-error --max-time 30 "${URL}{{HEALTH_PATH}}" >/dev/null
          curl --fail --silent --show-error --max-time 30 --output /dev/null "$URL"

      - name: Write the deployment evidence artifact
        env:
          URL: ${{ steps.service.outputs.url }}
        run: |
          set -euo pipefail
          DIGEST="$(gcloud run services describe "$SERVICE" \
            --region "$REGION" --project "$GCP_PROJECT_ID" \
            --format 'value(status.latestReadyRevisionName)')"
          # Non-secret facts only: no headers, no tokens, no environment dump.
          cat > integration-deployment.json <<JSON
          {
            "pr_number": "${PR_NUMBER}",
            "head_sha": "${HEAD_SHA}",
            "review_tag": "${{ needs.validate.outputs.review_tag }}",
            "run_url": "${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}",
            "service_url": "${URL}",
            "revision": "${DIGEST}",
            "image": "${IMAGE}"
          }
          JSON

      - name: Upload the evidence artifact
        uses: actions/upload-artifact@v4
        with:
          name: integration-pr-${{ needs.validate.outputs.pr_number }}-${{ needs.validate.outputs.short_sha }}
          path: integration-deployment.json
```

- [ ] **Step 5: Write the marker SQL**

`skills/delivery/cloud-run-continuous-deploy/templates/integration-marker.sql`:

```sql
-- One-time, idempotent initializer for the dedicated integration database.
--
-- Run by an operator, never by a workflow: an automated marker write would let a
-- misconfigured deploy mark production as disposable and then reset it.
--
-- The marker lives outside the schema the reset drops. Placed inside it, the
-- reset would destroy its own authorisation on the first run.
CREATE SCHEMA IF NOT EXISTS review_control;

CREATE TABLE IF NOT EXISTS review_control.environment_marker (
    environment text PRIMARY KEY
);

INSERT INTO review_control.environment_marker (environment)
VALUES ('integration')
ON CONFLICT DO NOTHING;
```

- [ ] **Step 6: Verify GREEN**

```bash
uv run pytest tests/test_cloud_run_deploy_skill.py -v
python3 -c "import sys, yaml" 2>/dev/null && echo "yaml available" || echo "skip yaml lint"
```

Expected: every test except the `deploy.yml` parametrized case passes. `deploy.yml` lands in Task 7.

- [ ] **Step 7: Commit**

```bash
git add skills/delivery/cloud-run-continuous-deploy/templates/integration-tag.yml \
        skills/delivery/cloud-run-continuous-deploy/templates/deploy-integration.yml \
        skills/delivery/cloud-run-continuous-deploy/templates/integration-marker.sql \
        tests/test_cloud_run_deploy_skill.py
git commit -m "feat(skills): add review tag trust boundary templates"
```

---

### Task 7: Branch deployment workflow

**Files:**
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/deploy.yml`
- Modify: `tests/test_cloud_run_deploy_skill.py`

**Interfaces:**
- Consumes: `_template()` from Task 5.
- Produces: workflow `deploy.yml`, triggered by pushes to the two configured branches, with an optional `workflow_dispatch` input that is cross-checked against the ref and never substituted for it.

- [ ] **Step 1: Add the failing tests**

Append to `tests/test_cloud_run_deploy_skill.py`:

```python
def test_environment_derives_from_the_ref_not_an_input():
    # Invariant 3: `environment: ${{ inputs.environment }}` hands production
    # secrets to anyone with workflow_dispatch rights.
    workflow = _template("deploy.yml")
    environment_lines = [
        line for line in workflow.splitlines() if line.strip().startswith("environment:")
    ]
    assert environment_lines, "deploy.yml declares no environment"
    for line in environment_lines:
        assert "github.ref_name" in line, line
        assert "inputs." not in line, line


def test_dispatch_input_is_cross_checked_against_the_ref():
    # An input may narrow what a ref permits; it may never widen it.
    workflow = _template("deploy.yml")
    assert "REQUESTED_ENVIRONMENT" in workflow
    assert "Deployments require" in workflow


def test_branch_images_are_tagged_by_sha():
    # Invariant 9.
    workflow = _template("deploy.yml")
    assert ":latest" not in workflow
    assert "rev-parse --short=12" in workflow
```

- [ ] **Step 2: Verify RED**

Run: `uv run pytest tests/test_cloud_run_deploy_skill.py -v -k "environment_derives or dispatch_input or branch_images"`

Expected: three failures with `FileNotFoundError`.

- [ ] **Step 3: Write the template**

`skills/delivery/cloud-run-continuous-deploy/templates/deploy.yml`:

```yaml
name: Deploy

on:
  push:
    branches: [{{DEV_BRANCH}}, {{PROD_BRANCH}}]
  workflow_dispatch:
    inputs:
      environment:
        description: "Environment to deploy (must match the branch you run from)"
        type: choice
        options: [dev, prod]
        required: true

concurrency:
  group: deploy-${{ github.ref_name }}
  cancel-in-progress: true

env:
  REGION: {{REGION}}
  AR_REPO: {{AR_REPO}}

jobs:
  deploy:
    runs-on: ubuntu-latest
    env:
      GCP_PROJECT_ID: ${{ secrets.GCP_PROJECT_ID }}
    permissions:
      id-token: write
      contents: read
    # Bound to the checked-out ref, never to a dispatch input. An input-selected
    # environment would hand production secrets to anyone who can press Run.
    environment: ${{ github.ref_name == '{{DEV_BRANCH}}' && 'dev' || github.ref_name == '{{PROD_BRANCH}}' && 'prod' || 'invalid' }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Validate deployment target
        id: env
        env:
          REQUESTED_ENVIRONMENT: ${{ inputs.environment || '' }}
        run: |
          set -euo pipefail
          [[ "$GCP_PROJECT_ID" =~ ^[a-z0-9-]+$ ]] ||
            { echo "GCP_PROJECT_ID is malformed" >&2; exit 1; }

          # The input may only agree with the ref. It can never choose.
          case "${{ github.event_name }}:$GITHUB_REF_NAME:$REQUESTED_ENVIRONMENT" in
            push:{{DEV_BRANCH}}:|workflow_dispatch:{{DEV_BRANCH}}:dev) ENV=dev ;;
            push:{{PROD_BRANCH}}:|workflow_dispatch:{{PROD_BRANCH}}:prod) ENV=prod ;;
            *)
              echo "Deployments require {{DEV_BRANCH}} -> dev or {{PROD_BRANCH}} -> prod;" \
                   "received ref $GITHUB_REF_NAME and environment ${REQUESTED_ENVIRONMENT:-none}" >&2
              exit 1
              ;;
          esac

          echo "env=$ENV" >> "$GITHUB_OUTPUT"
          echo "runtime_sa=${AR_REPO}-runtime-${ENV}" >> "$GITHUB_OUTPUT"
          echo "sha=$(git rev-parse --short=12 HEAD)" >> "$GITHUB_OUTPUT"

      - name: Authenticate to Google Cloud
        id: auth
        uses: google-github-actions/auth@v2
        with:
          token_format: access_token
          create_credentials_file: true
          project_id: ${{ secrets.GCP_PROJECT_ID }}
          workload_identity_provider: ${{ secrets.WIF_PROVIDER }}
          service_account: {{AR_REPO}}-deployer@${{ secrets.GCP_PROJECT_ID }}.iam.gserviceaccount.com

      - name: Set up gcloud
        uses: google-github-actions/setup-gcloud@v2
        with:
          project_id: ${{ secrets.GCP_PROJECT_ID }}

      - name: Log in to Artifact Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGION }}-docker.pkg.dev
          username: oauth2accesstoken
          password: ${{ steps.auth.outputs.access_token }}

      - name: Build and push the image
        run: |
          set -euo pipefail
          ENV="${{ steps.env.outputs.env }}"
          IMAGE="${REGION}-docker.pkg.dev/${GCP_PROJECT_ID}/${AR_REPO}/app:${ENV}-${{ steps.env.outputs.sha }}"
          docker build -f deploy/Dockerfile -t "$IMAGE" .
          docker push "$IMAGE"
          echo "IMAGE=$IMAGE" >> "$GITHUB_ENV"

      - name: Sync secrets to Secret Manager
        env:
          {{SECRET_ENV_BLOCK}}
        run: |
          set -euo pipefail
          ENV="${{ steps.env.outputs.env }}"
          sync_secret() {
            local name="$1" value="$2"
            if [[ -z "$value" ]]; then
              echo "refusing to sync empty value into ${name}" >&2
              exit 1
            fi
            printf '%s' "$value" | gcloud secrets versions add "$name" \
              --data-file=- --project "$GCP_PROJECT_ID"
          }
          {{SYNC_SECRET_CALLS}}

      - name: Run migrations
        run: |
          set -euo pipefail
          ENV="${{ steps.env.outputs.env }}"
          gcloud run jobs deploy "migrate-${ENV}" \
            --image "$IMAGE" \
            --region "$REGION" \
            --project "$GCP_PROJECT_ID" \
            --service-account "${{ steps.env.outputs.runtime_sa }}@${GCP_PROJECT_ID}.iam.gserviceaccount.com" \
            --command {{MIGRATE_ENTRYPOINT}} \
            --max-retries 0 \
            --set-secrets "{{MIGRATE_SECRETS}}"
          gcloud run jobs execute "migrate-${ENV}" \
            --region "$REGION" --project "$GCP_PROJECT_ID" --wait

      - name: Deploy the service
        id: service
        run: |
          set -euo pipefail
          ENV="${{ steps.env.outputs.env }}"
          gcloud run deploy "${AR_REPO}-${ENV}" \
            --image "$IMAGE" \
            --region "$REGION" \
            --project "$GCP_PROJECT_ID" \
            --allow-unauthenticated \
            --service-account "${{ steps.env.outputs.runtime_sa }}@${GCP_PROJECT_ID}.iam.gserviceaccount.com" \
            --cpu 1 --memory 1Gi --min 0 --max {{MAX_INSTANCES}} \
            --set-env-vars "DEPLOY_ENV=${ENV}" \
            --set-secrets "{{RUNTIME_SECRETS}}"
          URL="$(gcloud run services describe "${AR_REPO}-${ENV}" \
            --region "$REGION" --project "$GCP_PROJECT_ID" --format 'value(status.url)')"
          echo "url=$URL" >> "$GITHUB_OUTPUT"

      - name: Smoke test
        env:
          URL: ${{ steps.service.outputs.url }}
        run: |
          set -euo pipefail
          curl --fail --silent --show-error --max-time 30 "${URL}{{HEALTH_PATH}}" >/dev/null

      - name: Summary
        run: |
          {
            echo "## Deploy summary"
            echo "- environment: ${{ steps.env.outputs.env }}"
            echo "- url: ${{ steps.service.outputs.url }}"
            echo "- commit: ${{ steps.env.outputs.sha }}"
          } >> "$GITHUB_STEP_SUMMARY"
```

- [ ] **Step 4: Verify GREEN**

Run: `uv run pytest tests/test_cloud_run_deploy_skill.py -v`

Expected: all tests pass, including every parametrized `test_no_template_creates_a_service_account_key` case.

- [ ] **Step 5: Commit**

```bash
git add skills/delivery/cloud-run-continuous-deploy/templates/deploy.yml tests/test_cloud_run_deploy_skill.py
git commit -m "feat(skills): add branch deployment workflow template"
```

---

### Task 8: Single-container assets

**Files:**
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/nginx.conf`
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/entrypoint.sh`
- Create: `skills/delivery/cloud-run-continuous-deploy/templates/Dockerfile.combined`
- Modify: `tests/test_cloud_run_deploy_skill.py`

**Interfaces:**
- Consumes: `_template()` from Task 5.
- Produces: the `deploy/` directory contents referenced by `docker build -f deploy/Dockerfile` in Tasks 6 and 7.

- [ ] **Step 1: Add the failing tests**

Append to `tests/test_cloud_run_deploy_skill.py`:

```python
def test_entrypoint_uses_bash_for_wait_n():
    # `wait -n` is a bashism. Under dash it fails at runtime, inside a container,
    # on the first deploy -- the most expensive place to discover it.
    entrypoint = _template("entrypoint.sh")
    assert entrypoint.startswith("#!/usr/bin/env bash")
    assert "wait -n" in entrypoint


def test_entrypoint_exits_when_any_child_exits():
    # A supervisor that restarts children in place keeps a broken revision
    # serving traffic; Cloud Run must be allowed to replace it.
    entrypoint = _template("entrypoint.sh")
    assert "trap" in entrypoint
    assert re.search(r"wait -n\s*\n\s*exit", entrypoint)


def test_envsubst_is_restricted_to_port():
    # Unrestricted envsubst eats nginx's own $host and $remote_addr, and the
    # resulting config is silently wrong rather than broken.
    entrypoint = _template("entrypoint.sh")
    assert "envsubst '${PORT}'" in entrypoint


def test_nginx_listens_on_the_cloud_run_port():
    conf = _template("nginx.conf")
    assert "listen       ${PORT}" in conf or "listen ${PORT}" in conf


def test_nginx_forwards_the_proxy_headers():
    # Without X-Forwarded-Proto the apps generate http:// URLs behind Cloud Run's
    # TLS terminator, which breaks redirects and cookies.
    conf = _template("nginx.conf")
    assert "X-Forwarded-Proto" in conf
    assert "X-Forwarded-For" in conf
```

- [ ] **Step 2: Verify RED**

Run: `uv run pytest tests/test_cloud_run_deploy_skill.py -v -k "entrypoint or envsubst or nginx"`

Expected: five failures with `FileNotFoundError`.

- [ ] **Step 3: Write nginx.conf**

`skills/delivery/cloud-run-continuous-deploy/templates/nginx.conf`:

```nginx
# Rendered at container start by entrypoint.sh, which substitutes ${PORT} only.
server {
    listen       ${PORT};
    server_name  _;

    # Cloud Run terminates TLS and sets X-Forwarded-Proto. Pass it through, or the
    # applications generate http:// URLs and their redirects and cookies break.
    proxy_set_header Host              $host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $http_x_forwarded_proto;

    proxy_http_version 1.1;
    proxy_read_timeout 300s;
    client_max_body_size 32m;

    location {{API_PREFIX}}/ {
        proxy_pass http://127.0.0.1:{{API_PORT}};
    }

    location / {
        proxy_pass http://127.0.0.1:{{WEB_PORT}};
    }
}
```

- [ ] **Step 4: Write entrypoint.sh**

`skills/delivery/cloud-run-continuous-deploy/templates/entrypoint.sh`:

```bash
#!/usr/bin/env bash
# Run every HTTP service plus nginx in one Cloud Run container.
#
# bash, not sh: `wait -n` is a bashism and dash fails on it at runtime, inside
# the container, on the first deploy. The image must therefore install bash.
set -euo pipefail

: "${PORT:=8080}"
export PORT

# Restricted to ${PORT} on purpose. Unrestricted, envsubst also eats nginx's own
# $host, $remote_addr and $proxy_add_x_forwarded_for, and the rendered config is
# silently wrong rather than obviously broken.
envsubst '${PORT}' \
  < /etc/nginx/templates/default.conf.template \
  > /etc/nginx/conf.d/default.conf

# Kill the whole process group on the way out, so no child outlives the container.
trap 'kill 0' EXIT INT TERM

{{API_COMMAND}} &
{{WEB_COMMAND}} &
nginx -g 'daemon off;' &

# Returns as soon as the FIRST child exits. A dead application therefore takes
# the container down and Cloud Run replaces the revision, instead of the
# container staying up while serving errors from half of its routes.
wait -n
exit $?
```

- [ ] **Step 5: Write Dockerfile.combined**

`skills/delivery/cloud-run-continuous-deploy/templates/Dockerfile.combined`:

```dockerfile
# Single container: every HTTP service plus nginx on Cloud Run's $PORT.
# Build from the repository root: docker build -f deploy/Dockerfile .

FROM {{API_BASE_IMAGE}} AS api-build
WORKDIR /src/api
COPY {{API_CONTEXT}}/ ./
RUN {{API_BUILD_COMMAND}}

FROM {{WEB_BASE_IMAGE}} AS web-build
WORKDIR /src/web
COPY {{WEB_CONTEXT}}/ ./
RUN {{WEB_BUILD_COMMAND}}

FROM {{RUNTIME_BASE_IMAGE}}

# bash for `wait -n` in the entrypoint, gettext for envsubst, nginx for routing.
RUN apt-get update \
 && apt-get install -y --no-install-recommends bash gettext-base nginx curl \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=api-build /src/api /app/api
COPY --from=web-build /src/web /app/web

COPY deploy/nginx.conf /etc/nginx/templates/default.conf.template
COPY deploy/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh \
 && rm -f /etc/nginx/sites-enabled/default

# Cloud Run injects PORT; 8080 is only the local default.
ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["/app/entrypoint.sh"]
```

- [ ] **Step 6: Verify GREEN**

```bash
uv run pytest tests/test_cloud_run_deploy_skill.py -v
bash -n skills/delivery/cloud-run-continuous-deploy/templates/entrypoint.sh
```

Expected: all tests pass; `bash -n` is silent.

- [ ] **Step 7: Commit**

```bash
git add skills/delivery/cloud-run-continuous-deploy/templates/nginx.conf \
        skills/delivery/cloud-run-continuous-deploy/templates/entrypoint.sh \
        skills/delivery/cloud-run-continuous-deploy/templates/Dockerfile.combined \
        tests/test_cloud_run_deploy_skill.py
git commit -m "feat(skills): add single-container nginx assets"
```

---

### Task 9: The skill prose

Everything before this task built the parts. This task is the skill: the procedure an agent follows and the reasoning that makes the invariants stick.

**Files:**
- Modify: `skills/delivery/cloud-run-continuous-deploy/SKILL.md`
- Create: `skills/delivery/cloud-run-continuous-deploy/reference.md`

**Interfaces:**
- Consumes: every template and script from Tasks 2–8, by exact filename.
- Produces: the operator-facing procedure. Nothing consumes it.

- [ ] **Step 1: Write SKILL.md**

Replace the Task 1 stub body, keeping the frontmatter unchanged:

````markdown
---
name: cloud-run-continuous-deploy
description: Prepare a repo with a docker-compose.yml for continuous deployment to Google Cloud Run — dev on merge to dev, prod on merge to main, and a resettable integration environment from an immutable PR review tag. Use when wiring a repo to Cloud Run, when a deploy workflow hands credentials to an untrusted ref, when review deployments need isolation from prod, or when secrets must reach GitHub Environments without passing through an agent.
---

# Cloud Run continuous deploy

Three triggers, three environments, one rule: **an untrusted ref never receives a credential.**

| Event | Environment | Workflow |
| --- | --- | --- |
| merge into `dev` | `dev` | `deploy.yml` |
| merge into `main` | `prod` | `deploy.yml` |
| tag `review/pr-<n>/<sha>` | `integration` | `integration-tag.yml` → `deploy-integration.yml` |

Read [reference.md](reference.md) before generating anything. It holds the twelve
invariants and the failure each one prevents. The templates encode them; the
reference is why you must not edit them into something more convenient.

## Phase 0 — Preconditions

Stop and report if any of these fail. Do not work around them.

```bash
gh auth status
command -v gcloud varlock docker python3
git rev-parse --verify origin/dev && git rev-parse --verify origin/main
ls .env.schema docker-compose.yml
```

## Phase 1 — Inventory and worksheet

```bash
scripts/inventory-compose.sh
```

It classifies every compose service as `http`, `job` or `externalize` and prints
a starter worksheet after a `--- worksheet ---` line. **Every value it emits is a
guess except the service names.** Walk them with the operator, then write the
confirmed result to `deploy/cloud-run.map.yml`.

Ask, in this order:

1. **Topology.** Prefer `single-container` — one nginx process fronting every
   HTTP service on Cloud Run's `$PORT`. It removes CORS entirely, along with the
   second service, its canonical-URL variant and its service account. Choose
   `per-service` only when two services need different scaling or resource
   profiles, one must stay private while another is public, or the base images
   cannot reasonably be combined.
2. **Path prefixes**, if single-container. Exactly one service takes `/`.
3. **Externalized services.** Name the managed replacement for each. Do not
   generate anything that assumes a compose-provided database still exists.
4. **`reset_entrypoint`.** Optional. Omit it and the integration environment
   runs migrations only — never generate a reset step pointing at a missing
   entrypoint.

## Phase 2 — Environments and secrets

You must never hold a secret value. Discover names, never values:

```bash
varlock load --agent --filter='@sensitive'
```

`--agent` redacts values and neither flag is `--format`, so this stays outside
the agent deny-glob. If you need something that command cannot tell you, **ask
the operator** — `.env.schema` is deny-listed on purpose and reading it around
the rule is not an option.

```bash
gh api -X PUT "repos/$REPO/environments/dev"
gh api -X PUT "repos/$REPO/environments/prod"
gh api -X PUT "repos/$REPO/environments/integration"
```

Copy `templates/sync-github-secrets.sh` to `scripts/`, then run it once per
environment. The operator populates Varlock first:

```bash
APP_ENV=dev         varlock run --inject vars -- scripts/sync-github-secrets.sh dev
APP_ENV=prod        varlock run --inject vars -- scripts/sync-github-secrets.sh prod
APP_ENV=integration varlock run --inject vars -- scripts/sync-github-secrets.sh integration
```

**The `integration` environment's secrets are suffixed `_INTEGRATION`.** This is
not a style choice — see invariant 7. Tell the operator that a later failure
naming `database-url-integration` means the GitHub Environment secret
`DATABASE_URL_INTEGRATION` is missing.

## Phase 3 — GCP bootstrap (the operator runs this)

Copy `templates/bootstrap-gcp.sh` to `scripts/`, substituting `{{AR_REPO}}`,
`{{DEV_BRANCH}}`, `{{PROD_BRANCH}}` and `{{SECRET_KEYS}}`. Then hand it over:

```bash
scripts/bootstrap-gcp.sh --project PROJECT --region REGION --repo ORG/REPO
```

You do not run this. It needs project-admin rights that no deploy identity may
hold. It prints `GCP_PROJECT_ID` and `WIF_PROVIDER` — set both as **repository**
secrets, not environment secrets.

## Phase 4 — Generate

Copy and substitute. Never overwrite: if a target path exists, stop and report it.

| Template | Target |
| --- | --- |
| `deploy.yml` | `.github/workflows/deploy.yml` |
| `integration-tag.yml` | `.github/workflows/integration-tag.yml` |
| `deploy-integration.yml` | `.github/workflows/deploy-integration.yml` |
| `review_tag.py` | `scripts/review_tag.py` (verbatim, no substitution) |
| `nginx.conf`, `entrypoint.sh`, `Dockerfile.combined` | `deploy/` (single-container only) |
| `integration-marker.sql` | `resources/integration-marker.sql` |

Then write `docs/cloud-deploy.md` recording: the Supabase or Cloud SQL instance
per environment, the one-time marker initialization command, the GitHub
Environment secret names, and the Secret Manager mapping from Phase 2.

## Phase 5 — Verify

Walk every invariant in [reference.md](reference.md) against the generated files.
Then the acceptance run, which is the only proof that counts:

1. Open a harmless PR into `dev`, merge it, confirm the `dev` deploy.
2. Merge `dev` into `main`, confirm the `prod` deploy.
3. Open a PR into `dev`, tag its head `review/pr-<n>/<sha>`, push the tag.
4. Confirm: the listener dispatches, `validate` passes without credentials, the
   integration environment deploys, smoke tests pass, and the evidence artifact
   carries the head SHA and the image reference.
5. Confirm the services scaled to zero afterwards.

## Do not

- Do not widen the WIF attribute condition to tag refs to "make the tag flow simpler". That is the attack.
- Do not add a check to `integration-tag.yml` and believe it protects anything.
- Do not let a `workflow_dispatch` input choose an environment.
- Do not create a service-account key. Ever, for any reason.
````

- [ ] **Step 2: Write reference.md**

`skills/delivery/cloud-run-continuous-deploy/reference.md` — one section per invariant, each stating the rule, the failure it prevents, and how to check it:

````markdown
# Reference: the twelve invariants

Each one guards a specific failure. The templates encode them; a template edit
that drops one is a bug, and `tests/test_cloud_run_deploy_skill.py` in the
scaffolding repo fails when it happens.

## 1. An untrusted ref never receives a credential

The review-tag path is three stages: an unprivileged listener, a credential-free
`validate` job, and a privileged `deploy` job that depends on it. Nothing that
runs before validation can reach WIF, an Environment, or a secret.

**Prevents:** a tag pointing at arbitrary code that builds and runs with deploy
credentials in scope.

**Check:** the block above `deploy:` in `deploy-integration.yml` contains no
`environment:` and no `google-github-actions/auth`.

## 2. The listener is attacker-controlled code

For `on: push: tags:`, GitHub loads the workflow file **from the tagged commit's
own tree**. Whoever creates the tag can rewrite every line of the listener,
including any check you add to it.

**Prevents:** the belief that validating in the listener is worth anything. It is
fail-fast ergonomics. The boundaries are invariant 4 and the re-validation in the
trusted workflow, and both must hold with the listener assumed hostile.

**Check:** the listener carries the comment saying so, and holds no secret,
environment, or federation step.

## 3. `environment:` derives from the ref, never from an input

The expression reads `github.ref_name`. A dispatch input may be cross-checked
against the ref — it may never be substituted for it.

**Prevents:** anyone with `workflow_dispatch` rights selecting `prod` from a
feature branch and receiving production secrets.

**Check:** every `environment:` line in `deploy.yml` mentions `github.ref_name`
and no line mentions `inputs.`.

## 4. The WIF condition pins repository and branch refs only

`assertion.repository == '<repo>' && (assertion.ref == 'refs/heads/dev' || assertion.ref == 'refs/heads/main')`

**Prevents:** a federated token reachable by anyone who can push a tag. This is
the reason the review-tag path runs its privileged half from the trusted branch
rather than from the tag.

**Check:** `bootstrap-gcp.sh` contains `refs/heads/` and does not contain
`refs/tags/`.

## 5. Untrusted heads are checked out defensively

Into a subdirectory, with `persist-credentials: false`, only after validation.
Trusted control code is checked out separately, from the trusted branch.

**Prevents:** a token in the `.git/config` of attacker-controlled code.

## 6. Resource inspection fails closed

A `describe` that fails for any reason other than a recognized not-found
signature aborts the script.

**Prevents:** a transient permission or network error reading as "absent", after
which the bootstrap cheerfully recreates — or resets — a live resource.

**Check:** `bootstrap-gcp.sh` contains `refusing to treat it as absent`.

## 7. Environment-suffixed secret names for the destructive environment

`integration` uses `<NAME>_INTEGRATION`; `dev` and `prod` keep bare names.

**Prevents:** the worst failure in the whole design. GitHub resolves a secret
reference environment → repository → organization. A bare `DATABASE_URL` missing
from the `integration` Environment resolves *silently* to a repository- or
organization-scoped secret of the same name — and that DSN is handed to a job
whose entire purpose is to drop a schema. Suffixed, a missing secret resolves to
the empty string and the workflow's guard fails loudly.

`dev` and `prod` run migrations, never a destructive reset, so they are not
exposed to this and keep the bare names.

**Operator note:** the failure names the Secret Manager destination
(`database-url-integration`), not the GitHub Environment secret
(`DATABASE_URL_INTEGRATION`). Document that mapping in the runbook.

## 8. Runtime identities get resource-scoped IAM only

One service account per environment. Secret access bound per secret, storage per
bucket, impersonation per account. No runtime identity holds a project-level role.

**Prevents:** an integration deployment reading production secrets because both
runtimes happen to hold `roles/secretmanager.secretAccessor` on the project.

## 9. Immutable image tags from the validated head SHA

`<env>-<sha12>`, or `integration-pr-<n>-<sha12>`. Never `latest`.

**Prevents:** a revision nobody can trace to a commit, and a redeploy that
silently ships different bytes.

## 10. A destructive reset is gated three ways

`DEPLOY_ENV=integration`, `ALLOW_DATABASE_RESET=true`, and a marker row in a
schema outside the one being dropped.

**Prevents:** resetting the wrong database. The two environment variables travel
with a misconfigured workflow; only the marker travels with the database. A
marker inside `public` would be destroyed by the reset it authorizes, so it lives
in `review_control`.

The marker is initialized once, by an operator, against the dedicated integration
database. Never automate it — an automated write lets a misconfigured deploy mark
production as disposable and then reset it.

## 11. Non-cancelling concurrency on shared environments

`concurrency: {group: integration, cancel-in-progress: false}`.

**Prevents:** a second review tag cancelling a run mid-reset and leaving a
half-migrated database that the next deploy then builds on.

## 12. Workload Identity Federation only

No service-account key is created, downloaded, stored in GitHub, or written to an
operator machine.

**Prevents:** a permanent, unrevoked credential sitting in a repo, a CI log, or a
laptop backup. WIF tokens expire; key files do not.
````

- [ ] **Step 3: Verify the whole suite**

```bash
uv run pytest tests/ -v
uv run ruff check .
uv run ruff format --check .
```

Expected: all tests pass, ruff clean.

- [ ] **Step 4: Verify the skill installs**

```bash
npx skills add . --agent opencode --yes --skill cloud-run-continuous-deploy --full-depth
ls .agents/skills/cloud-run-continuous-deploy/templates/
```

Expected: all ten templates present. The `--full-depth` flag is what carries
`scripts/` and `templates/` alongside `SKILL.md`; without it only the prose
installs and every phase after 1 fails at runtime.

- [ ] **Step 5: Commit**

```bash
git add skills/delivery/cloud-run-continuous-deploy/SKILL.md \
        skills/delivery/cloud-run-continuous-deploy/reference.md
git commit -m "docs(skills): add cloud-run-continuous-deploy procedure and invariants"
```

---

## Plan self-review

**Spec coverage:**

| Spec section | Task |
| --- | --- |
| §1.1 single container + nginx | 8 |
| §1.2 per-service fallback | 6, 7 (noted in templates), 9 (Phase 1 question) |
| §1.3 service classification | 3 |
| §2 mapping worksheet + derived names | 3, 9 |
| §3.1–3.3 Varlock bridge | 4 |
| §3.4 `_INTEGRATION` suffix | 4, 6, 9 |
| §3.5 environment creation | 9 |
| §4 invariants 1–12 | 5, 6, 7, 8 (tests); 9 (reference) |
| §5 reset contract | 6 |
| §6.1 layout | 1–9 |
| §6.2 procedure | 9 |
| §6.3 clean-adds only | 9 (Phase 4) |
| §7 registration | 1 |
| §7.1 new tests | 5, 6, 7, 8 |

No gaps.

**Type consistency:** `parse_review_tag` / `validate` / `ReviewTag` are named
identically in Task 2's tests, implementation and CLI, and in the workflow
invocations in Task 6. `_template()` and `TEMPLATES` are defined in Task 5 and
reused unchanged in Tasks 6–8. Template placeholder names (`{{AR_REPO}}`,
`{{DEV_BRANCH}}`, `{{PROD_BRANCH}}`, `{{REGION}}`, `{{INTEGRATION_REGION}}`,
`{{SECRET_KEYS}}`, `{{SECRET_ENV_BLOCK}}`, `{{SYNC_SECRET_CALLS}}`,
`{{RUNTIME_SECRETS}}`, `{{MIGRATE_SECRETS}}`, `{{RESET_SECRETS}}`,
`{{MIGRATE_ENTRYPOINT}}`, `{{RESET_ENTRYPOINT}}`, `{{HEALTH_PATH}}`,
`{{MAX_INSTANCES}}`, `{{REVIEW_TAG_PATTERN}}`, `{{API_PREFIX}}`, `{{API_PORT}}`,
`{{WEB_PORT}}`, `{{API_COMMAND}}`, `{{WEB_COMMAND}}`, `{{API_BASE_IMAGE}}`,
`{{API_CONTEXT}}`, `{{API_BUILD_COMMAND}}`, `{{WEB_BASE_IMAGE}}`,
`{{WEB_CONTEXT}}`, `{{WEB_BUILD_COMMAND}}`, `{{RUNTIME_BASE_IMAGE}}`) are
consistent across Tasks 5–8 and listed in Task 9's Phase 3 and Phase 4.

**Known ordering artifact:** the parametrized
`test_no_template_creates_a_service_account_key` cases fail from Task 5 until
Task 7 lands the last template. Each task's verification step says which cases
are expected to still fail, so a red run is never ambiguous.
