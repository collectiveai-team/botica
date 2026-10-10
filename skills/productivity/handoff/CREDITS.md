# Credits

Adapted from [Matt Pocock](https://github.com/mattpocock)'s
[`handoff` skill](https://github.com/mattpocock/skills/blob/v1.3.1/skills/productivity/handoff/SKILL.md)
in `mattpocock/skills`.

The original provides conversation handoffs, references to existing artifacts
instead of duplicating them, suggested skills, redaction, and tailoring to the
next session's focus.

The internal version adds explicit workspace/worktree and Git state, verification
results, remaining work, and a restart prompt. It stores handoffs under the
workspace's `.tmp/handoff/` rather than the operating system's temporary directory.
