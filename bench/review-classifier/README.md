# Classifier evaluation set

Run `botica review evaluate --dataset bench/review-classifier/cases.jsonl`.
Add `--observation <handoff.json>` for each recorded classification of a case.
The command never calls a model, changes a label or enables automation.

The initial seven cases use merged snapshots.
The merge commits of #220, #222, #226 and #284 were read from GitHub on 2026-11-18;
their parents are recorded by Git. #316–318 are merged maintenance examples from
`origin/main`. The three rejected-proposal labels cite their actual reverts. The
known bug-fix sample and maintenance samples remain unlabeled. A merge alone does
not show that automation was appropriate. No model probabilities are invented.

Each case needs an evidence-backed label and a current observation before it
contributes to the model measurement. Missing policy, check provenance, role
results or context stays unmeasured. A changed question digest or model invalidates
the affected observations. Cases have distinct heads; reruns cannot inflate the
sample. Noncritical changes are not routed to humans by an incomplete evaluation.

## Proposed activation evidence

The initial seed is not a calibrated set. The implemented readiness criterion is
at least 300 predicted automation candidates, at least 30 labeled negative cases
with otherwise-passing low-risk noncritical inputs, no false eligibility, and no
unlabeled, unmeasured or incompatible case. With zero false eligibility among
300 candidates, the one-sided 95% binomial upper bound is
`1 - 0.05^(1/300)`, about 0.9936%. This assumes representative independent cases.
Any observed false eligibility blocks readiness; its conservative upper bound is
reported as 1.

This measurement does not prove future safety. Enabling automation remains a
separate explicit decision, and its workflow is disabled. The seed cannot satisfy
the readiness criterion and is not presented as successful model evaluation.
