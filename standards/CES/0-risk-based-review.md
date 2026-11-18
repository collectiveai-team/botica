---
id: CES-0
title: Review depth follows the risk of the diff
---

## Directive

1. Botica assesses branch risk from every best common ancestor to the head, and integration
   risk from the destination tip to the integration result. Effective risk is the higher completed
   assessment. Paths and supported detector facts set the floor under destination-owned policy.
   Conservative policy bounds keep unknown facts explicit; they do not assert that a detector
   completed. A label, description or statement of the author cannot lower the floor.
2. The configuration lists the paths of four areas. `critical` holds authentication, billing,
   migrations and permissions. `platform` holds CI, build, release, infrastructure as code and the
   files that decide how a pull request is reviewed or that an agent loads as instructions. `core`
   holds the domain and the public contracts. `periphery` holds documentation, tests and the
   patch updates of dependencies that ADR 0018 defines. A file that the configuration does not
   list for an area belongs to `core`. A file that belongs to several areas takes the highest
   floor, and the files that decide the review or that an agent loads are `platform` whatever
   the configuration says. A pull request touches every area that one of its files belongs to.
3. Only a pull request that touches `critical` requires human review by the owner of the area.
   A model, rule or bot cannot lift that requirement. Other areas, non-low risk, size and technical
   failures do not require a person. They can select more roles or block automation. Any future
   automation requires complete current evidence and an evaluated destination policy. The
   implemented automation workflow remains disabled.
4. A bot can approve a pull request, and a bot approval is not a person's approval. Where an area
   asks for approvals, only the approvals of people count, and the author of the pull request is
   not one of them.
5. One person answers for each pull request: the person who directed the agent that wrote it, or
   the author when no agent wrote it. That person reads the diff before asking for review,
   explains each change and answers the review comments. The agent answers for nothing.
6. An approval belongs to one commit. A new commit asks for approval again and classifies the
   pull request again.
7. A workflow that reviews or classifies a pull request with a model runs only when its author is
   trusted. A trusted author has write access to the repository and opened the pull request from
   a branch of the repository. A pull request from a fork, or from an author without write
   access, cannot receive credential-bearing model execution. That restriction blocks execution
   or automation, without requiring a person unless the change touches `critical`. No workflow
   gives it a secret, and a workflow that holds a secret does not run its code.

## Why

A review takes the time of a person, and the risk differs between changes: a typo in a page and a
change to the release workflow wait in the same queue today. Paths give a floor that the author
cannot change. If the author set the risk, the pull request that asks for no second reader could
be the one that needs it.

A bot and a model both act on the text they read. The owner of an area and the person who
directed the agent can say why a change is right. An approval records what that person looked at,
and a commit pushed after it is a change that nobody looked at.

ADR 0017 and ADR 0018 apply this standard. They define the areas, the floor of each tier, the
classifier, the gates and the approval bound to the head commit.

The workflow `dependabot-automerge-run.yml` merges a pull request with the label `automerge` once
its checks pass. It reads no area and no gate of ADR 0018, so it departs from directive 3. ADR
0018 decision 12 leaves it as it is until a later change routes it through the gates.

## Examples

The table assumes a configuration that lists `website/docs/` as `periphery`, `db/migrations/**`
as `critical` and `core/` as `core`.

| The diff touches | Area | Who reviews |
|---|---|---|
| a page of `website/docs/` | `periphery` | no person, when ADR 0018 lets it merge |
| a patch update of a package that the manifest lists, from the same source and the same constraint, released at least as long ago as the cooldown | `periphery` | no person, when ADR 0018 lets it merge |
| a minor update of a dependency | the area of its manifest, with the floor raised to `medium` by the `new-dependency` signal | planned roles, no human requirement unless critical |
| `.github/workflows/ci.yml` | `platform` | planned roles, no human requirement |
| `db/migrations/0042_add_index.sql` | `critical` | the owner of the area |
| a public function in `core/` | `core` | planned roles, no human requirement |
| a page of `website/docs/` and `.github/workflows/ci.yml` | `periphery` and `platform` | planned roles, no human requirement |

A pull request whose description says "docs only" and that also edits `.github/workflows/ci.yml`
is a `platform` change.

A person opens a pull request with an agent, and a bot approves it. It waits for a person where
the area asks for one. After a push to that pull request, the approvals that came before it no
longer count.
