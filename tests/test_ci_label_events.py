"""Label-only CI runs cannot cancel an active review suite."""

import json
import re
import subprocess
from itertools import product
from pathlib import Path


def _label_job_conditions(workflow: str) -> dict[str, str]:
    jobs: dict[str, str] = {}
    for name, body in re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)", workflow, re.M | re.S):
        condition = re.search(r"^    if: (.+)$", body, re.MULTILINE)
        if condition is not None and "github.event.action" in condition[1]:
            jobs[name] = condition[1]
    return jobs


def _expected_label_case(action: str, label: str, jobs: dict[str, str]) -> dict[str, object]:
    resumes = action == "unlabeled" and label == "botica:stack-member"
    normal = action in ("opened", "synchronize") or resumes
    acknowledged = label in ("contract-change", "review-flow-standards")
    contract = normal or acknowledged or label == "botica:stack-member"
    group = "contract-label" if acknowledged else "ignored-label"
    return {
        "action": action,
        "label": label,
        "group": "review" if normal else group,
        "jobs": {name: contract if name == "rule-contract" else normal for name in jobs},
    }


def test_ci_label_events_preserve_review_runs_and_select_only_intended_jobs():
    workflow = (Path(__file__).resolve().parents[1] / ".github/workflows/ci.yml").read_text()
    group = re.search(r"^  group: (.+)$", workflow, re.MULTILINE)
    assert group is not None
    expression = group[1].rsplit("${{", 1)[1].split("}}", 1)[0].strip()
    jobs = _label_job_conditions(workflow)
    assert {"test", "references", "rule-contract", "lint", "review-policy"} == jobs.keys()
    events = product(
        ("opened", "synchronize", "labeled", "unlabeled"),
        ("botica:stack-member", "contract-change", "review-flow-standards", "other"),
    )
    cases = [_expected_label_case(action, label, jobs) for action, label in events]
    script = """
const input = JSON.parse(process.argv[1]);
const contains = (values, value) => values.includes(value);
for (const item of input.cases) {
  const github = {event_name: 'pull_request', event: {action: item.action,
    label: {name: item.label}, pull_request: {draft: false, labels: []}}};
  const evaluate = expression => Function('github', 'contains', 'fromJSON',
    'return (' + expression + ')')(github, contains, JSON.parse);
  const group = evaluate(input.expression);
  if (group !== item.group) throw Error(JSON.stringify({item, actualGroup: group}));
  for (const [name, condition] of Object.entries(input.jobs)) {
    // GitHub's label projection is empty in this non-deferred fixture.
    const actual = evaluate(condition.replace('labels.*.name', 'labels'));
    if (actual !== item.jobs[name]) throw Error(JSON.stringify({item, name, actual}));
  }
}
"""
    result = (
        subprocess.run(  # botica: ignore[test-no-subprocess] -- evaluate actual workflow predicates
            [
                "node",
                "-e",
                script,
                json.dumps({"expression": expression, "jobs": jobs, "cases": cases}),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
    )
    assert result.returncode == 0, result.stderr
