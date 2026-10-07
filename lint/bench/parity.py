"""Compare botica's embedded ast-grep rules with the ast-grep CLI on the same tree.

Usage: python parity.py <repo_dir> <botica_bin> <ast_grep_rules_dir>
Compares (path, rule id, 1-based line, 1-based column) for every rule the two share.
"""

import json
import subprocess
import sys
from pathlib import Path

repo, botica, rules_dir = Path(sys.argv[1]).resolve(), sys.argv[2], Path(sys.argv[3]).resolve()
ast_grep_rule_ids = {p.stem for p in (rules_dir / "rules").glob("*.yml")}

sg = subprocess.run(
    ["ast-grep", "scan", "--config", str(rules_dir / "sgconfig.yml"), "--json=stream", str(repo)],
    capture_output=True,
    text=True,
    check=False,
)
expected = set()
# Split on \n only: str.splitlines() also breaks on U+2028/U+0085, which minified JS contains.
for line in filter(None, sg.stdout.split("\n")):
    hit = json.loads(line)
    path = Path(hit["file"]).resolve().relative_to(repo).as_posix()
    expected.add(
        (
            path,
            hit["ruleId"],
            hit["range"]["start"]["line"] + 1,
            hit["range"]["start"]["column"] + 1,
        )
    )

mine = subprocess.run([botica, "lint", "."], cwd=repo, capture_output=True, text=True, check=False)
actual = set()
for line in filter(None, mine.stdout.split("\n")):
    location, _, rest = line.partition(": ")
    path, line_no, col = location.rsplit(":", 2)
    rule_id = rest.split("[", 1)[1].split("]", 1)[0]
    if rule_id in ast_grep_rule_ids:
        actual.add((path.removeprefix("./"), rule_id, int(line_no), int(col)))

print(f"ast-grep CLI: {len(expected)} hits | botica: {len(actual)} hits")
print(f"only in ast-grep: {len(expected - actual)} | only in botica: {len(actual - expected)}")
for label, diff in (("ONLY ast-grep", expected - actual), ("ONLY botica", actual - expected)):
    for item in sorted(diff)[:10]:
        print(f"  {label}: {item}")
sys.exit(0 if expected == actual else 1)
