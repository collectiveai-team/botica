# CES-118 · copy-paste duplication (jscpd)

**Code:** `CES-118` &nbsp;·&nbsp; **Slug:** `code-duplication-jscpd` &nbsp;·&nbsp; **Enforced
by:** prek hook &nbsp;·&nbsp; **Tracker:**
[#118](https://github.com/collectiveai-team/scaffolding/issues/118)

## Directive

No block of code above [`jscpd`](https://github.com/kucherenko/jscpd)'s token threshold may be
duplicated (near-verbatim) elsewhere in the repo without an explicit exemption. Enforced via a
repo-wide duplication-percentage threshold (`--threshold`) that fails the hook on regression.

## Why

Copy-pasted logic is a common, lazy AI-agent failure mode: an agent asked to add a similar feature
copies the nearest existing block instead of extracting a shared helper, so a later bug fix in one
copy silently doesn't apply to the other. Nothing else in the house stack (ruff, ast-grep,
import-linter) detects this — ast-grep matches a single declared pattern, not "this block looks
like that other block." `jscpd` is polyglot (tree-sitter, 223+ formats) so it also covers a
frontend/Docker/YAML portion of a scaffolded repo, not just Python.

## Threshold and scope

Ships at `--threshold 1` (percent) in the generic template — calibrate per repo. `jscpd` respects
`.gitignore` by default (no `--gitignore` flag needed on v5). Use `-i`/`--ignore` glob patterns to
exempt deliberately-mirrored content (e.g. this tool's own `guide.md`, which intentionally mirrors
`README.md`'s bootstrap instructions for the agentic-install flow) rather than raising the global
threshold to paper over unrelated real duplication.

`--threshold` alone is the pass/fail gate (exits non-zero only when duplication exceeds the
percentage). jscpd v5 also ships a separate `--exit-code [<code>]` flag described as "exit with
code if duplicates found" — **do not** add it alongside `--threshold`: it fails on *any* clone at
all, independent of the percentage, which silently makes the threshold inert. Verified directly by
running both flag combinations against this repo.

## When you hit a violation

1. Find the shared logic driving the clone and extract it into a named function/module.
2. If the duplication is deliberate (e.g. two docs meant to mirror each other), exempt the path
   via `-i`/`--ignore`, with a comment explaining *why* it's deliberate.
3. Re-run the hook to confirm the repo is back under threshold.

## Migration (existing repos)

Run `npx jscpd@5 . --threshold 1` once to baseline. Either fix real duplication or
calibrate the threshold/ignore list to the repo's current, understood state — document any
deliberate exemption inline, same "flag, don't silently exempt" posture as `file-size-guard`
(CES-71).

## Suppression

Path-level: `-i`/`--ignore` glob patterns on the hook's `entry`. Code-level: `--ignore-pattern`
regex to skip specific token spans (e.g. license headers). Never raise the global threshold to
mask a specific, understood duplicate — scope the exemption instead.
