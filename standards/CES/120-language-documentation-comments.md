---
id: CES-120
title: Language, documentation and comments
---

## Directive

1. Text states what a thing is and what it does, in plain sentences that a file, a command or an
   issue of the repository backs. It applies to documentation, standards, ADRs, commit messages,
   messages of rules and comments in code. The text carries none of the signs below.
2. Code explains itself through its names and its structure. A comment is written only for what the
   code cannot say: the reason for a choice that looks wrong, an invariant, or a workaround with the
   link to its cause.
3. The project names each concept of its domain with one term. The glossary of the project
   (`GLOSSARY.md`) lists each term with its definition and the words to avoid for it, and the code
   and the documentation use the term.

### Signs of text written by a model

The list follows the Wikipedia guide "Signs of AI writing"
(<https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing>).

| Sign | Examples |
|---|---|
| Inflated significance and promotion | key, crucial, pivotal, robust, seamless, powerful, vibrant, landscape, testament, "plays a vital role" |
| A trailing clause that judges | "…, ensuring …", "…, highlighting …", "…, which keeps …", "…, making it …" |
| Words that avoid "is" and "has" | serves as, acts as, functions as, features, offers, provides, boasts |
| Definition by negation | "X does not …", "not X but Y", "Y rather than X", "no A, no B, just C" |
| Lists of three | three adjectives or three short phrases in a row |
| Vague attribution and hedges | "experts say", "some", "often", "typically", "it is worth noting" |
| Vocabulary of models | delve, tapestry, intricate, underscore, garner, showcase, leverage, foster, enhance |
| Format | bold for emphasis, a bold lead-in on each list item, spaced em dashes, title case in headings, emoji, a closing paragraph that repeats the section |
| Text addressed to the reader | "I hope this helps", "let me know", "certainly", "great question" |
| A claim with no source | a number, a count of repositories or a "most teams do" that no file backs |

### Comments

| Write a comment for | Do not write a comment that |
|---|---|
| the reason for a choice a reader would undo | repeats what the next line does |
| an invariant the types do not show | names a section of a file (`# --- helpers ---`) |
| a workaround, with the link to the bug | keeps old code commented out |
| a public interface, in its docstring | tells the history of the change (git has it) |

## Why

A reader of a page or of a comment acts on what it says. A sign of model text is a sentence the
reader has to discount, and a comment that repeats the code is a second copy that drifts from it.
One term per concept lets a reader search for a word and find every place that means it.
