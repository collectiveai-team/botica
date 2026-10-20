## Directive

Documentation, standards, ADRs and other prose say what a thing is and what it does, in plain sentences that a file, a command or an issue of the repository backs, with none of the signs of model writing that CES-120 lists.

## Why

A reader acts on what a page says. A sentence that inflates, hedges or claims with no source is one the reader has to discount, and it hides the sentences that matter.

## Flag

- A word of inflated significance: key, crucial, pivotal, robust, seamless, powerful, landscape.
- A clause at the end of a sentence that judges it: "…, ensuring …", "…, which keeps …".
- "serves as", "acts as", "features", "offers" where "is" or "has" says it.
- A definition by negation: "X does not …", "not X but Y", "Y rather than X".
- A list of three adjectives or short phrases.
- A hedge or a vague attribution: "typically", "some", "experts say".
- Bold for emphasis, a bold lead-in on each item of a list, a spaced em dash.
- Text addressed to the reader: "I hope this helps", "let me know".
- A number or a count that no file of the repository backs.

## Do not flag

- A quotation, which keeps its words.
- A negation that states a fact a reader needs: "the hook does not run on merge commits".
- Bold in a table header or a term defined in a glossary.

## Confirm

Before reporting a claim with no source, look for the file, the command or the issue that backs it.

## Do

```markdown
The hook runs `botica lint` on the files of the commit. It fails when a finding is an error.
```

## Don't

```markdown
The hook serves as a **robust** safety net, ensuring that every commit meets the highest standards.
```

## Suppression

A file that quotes model text on purpose, such as this README, takes `botica: ignore-file[plain-writing] -- <reason>` in a comment. A folder goes in the `extend-ignores` of the rule.
