---
role: refute
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
owners: ["@jedzill4"]
max-findings: 50
---

## Invariants

- The entry points of botica are the subcommands of `botica` (`crates/botica-cli/src/main.rs`) and
  their flags, `botica server` and the language server of `botica-lsp`, the extension protocol and
  its SDKs under `sdk/`, the Python module `botica.sdk` of the wheel, the recipes of
  `catalog/recipes/` and the files they install, the workflows of `.github/workflows/` and their
  triggers, the hooks of `prek.toml`, the scripts that a workflow or a hook runs, and the skills
  under `skills/`, which an agent runs. The crates are not published (`publish = false` in
  `Cargo.toml`), so a public function of a crate is reachable only through one of these.
- A rule of the catalog is reachable when it runs in `botica lint`: its `rule.toml` under
  `catalog/rules/` and its implementation are an entry point of their own.
- A finding of `spec` about a criterion that the issue asks for, and a finding of `tests` about a
  behaviour with no test or a weakened suite, are about what the diff should have done; they are
  never `pre-existing`.

## Check always

- Each finding of `P0` and `P1` gets an attempt against the files of the base or of `HEAD` (a
  command of `git` on the machine of the author, a read of the file in CI), not only a reading of
  its text.
- A finding refuted as `pre-existing` names the commit of the base where its lines already were,
  and says that no line of the diff changes what reaches them.
- A finding refuted as `duplicate` of a check names the hook of `prek.toml`, the job of
  `.github/workflows/` or the line of the role file that reports it.
- A finding that only a run of code could reproduce stays `confirmed`, and its `how_to_verify`
  has a step that a person runs.

## Do not report

- New findings. This role refutes; it does not review the diff again.

<!-- botica:covered -->
<!-- /botica:covered -->

## Evidence

Each refutation gives the command that the role ran or the file and line that disproves the
finding. `max-findings` is the number of findings that the role judges; `refute.json` keeps every
finding of the inputs.
