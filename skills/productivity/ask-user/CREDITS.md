# Credits

Adapted from [Matt Pocock](https://github.com/mattpocock)'s
[`ask-matt` skill](https://github.com/mattpocock/skills/blob/v1.3.1/skills/engineering/ask-matt/SKILL.md)
and its
[phase-boundary guidance](https://github.com/mattpocock/skills/blob/v1.3.1/skills/engineering/ask-matt/PHASE-BOUNDARIES.md)
in `mattpocock/skills`.

The main idea-to-implementation flow, on-ramps, standalone routes, and ordered
Continue / Clear / Handoff / Subagent / Compact decision tree come from that work.

The internal version is renamed `ask-user`, condensed to the installed catalog,
and includes the local engineering gate, refactoring, test review, journal,
handoff, and PR-body workflows. Recommendations require confirmation before
invoking another user-invoked skill.
