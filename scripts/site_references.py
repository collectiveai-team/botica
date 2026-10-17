"""Write the pages of the topics and list each topic on the pages that cite it.

`scripts/build_site.sh` runs it after `scripts/site_standards.py`, with the output of
`botica refs export`: every topic, with the text of the lines that each file reference shows. Each
topic becomes the page `references/<topic>.md` of the site, with its README, each reference (the
address of a page, or the lines of a file in a block of code) and the rules and standards that cite
it. The page of a rule or a standard that cites topics, by the `references` field of its front
matter, gets the list of them at the end.
"""

from __future__ import annotations

import json
import os
import re
from dataclasses import dataclass, field
from pathlib import Path

import cyclopts

app = cyclopts.App(name="site-references", help=__doc__)

FRONT_MATTER_END = "\n---\n"
LANGUAGES = {
    ".py": "python",
    ".rs": "rust",
    ".toml": "toml",
    ".md": "markdown",
    ".sh": "bash",
    ".js": "javascript",
    ".ts": "typescript",
    ".json": "json",
    ".yml": "yaml",
    ".yaml": "yaml",
}


@dataclass(frozen=True)
class Reference:
    """A file or a page that a topic points at: `url` or `path`, and the lines of a file."""

    note: str
    path: str = ""
    url: str = ""
    lines: str | None = None
    anchor: str | None = None
    repo: str | None = None
    rev: str | None = None
    license: str | None = None
    text: str = ""


@dataclass(frozen=True)
class Topic:
    id: str
    title: str
    readme: str
    references: list[Reference] = field(default_factory=list)


@dataclass(frozen=True)
class Citation:
    """A page of a rule or a standard that cites topics."""

    page: Path
    name: str
    topics: list[str]
    link: str
    sort: tuple[int, str, int]


def _split(text: str) -> tuple[list[str], str]:
    """Return the lines of the front matter of a page, and what follows it."""
    head, _, body = text.removeprefix("---\n").partition(FRONT_MATTER_END)
    return head.splitlines(), body


def _value(raw: str) -> object:
    return json.loads(raw) if raw.startswith(('"', "[")) else raw


def _fields(page: Path) -> dict[str, object]:
    lines, _ = _split(page.read_text(encoding="utf-8"))
    pairs = (line.partition(": ") for line in lines)
    return {key: _value(raw) for key, _, raw in pairs if raw}


def _cited(fields: dict[str, object]) -> list[str]:
    """Return the ids of the topics that the front matter of a page cites."""
    value = fields.get("references")
    return [str(item) for item in value] if isinstance(value, list) else []


def _reference(raw: dict[str, str | None]) -> Reference:
    return Reference(
        note=raw["note"] or "",
        path=raw["path"] or "",
        url=raw["url"] or "",
        lines=raw["lines"],
        anchor=raw["anchor"],
        repo=raw["repo"],
        rev=raw["rev"],
        license=raw["license"],
        text=raw["text"] or "",
    )


def _topics(export: Path) -> list[Topic]:
    """Return the topics of the output of `botica refs export`, by id."""
    parsed = json.loads(export.read_text(encoding="utf-8"))
    return [
        Topic(
            id=raw["id"],
            title=raw["title"],
            readme=raw["readme"],
            references=[_reference(reference) for reference in raw["references"]],
        )
        for raw in sorted(parsed["topics"], key=lambda raw: raw["id"])
    ]


def _link(target: Path, origin: Path) -> str:
    return Path(os.path.relpath(target, origin.parent)).as_posix()


def _citations(docs: Path) -> list[Citation]:
    """Return the rules and the standards that cite topics: rules by code, then standards."""
    found = []
    for page in sorted((docs / "rules").glob("*/*.md")):
        fields = _fields(page)
        if _cited(fields):
            found.append(
                Citation(
                    page=page,
                    name=f"`{fields['rule']}` ({fields['code']})",
                    topics=_cited(fields),
                    link=f"[`{fields['rule']}`]({{page}}) ({fields['code']})",
                    sort=(0, str(fields["code"]), 0),
                )
            )
    for page in sorted((docs / "standards").glob("*/*.md")):
        fields = _fields(page)
        if _cited(fields):
            prefix, _, number = str(fields["id"]).rpartition("-")
            found.append(
                Citation(
                    page=page,
                    name=str(fields["id"]),
                    topics=_cited(fields),
                    link=f"[{fields['id']}]({{page}})",
                    sort=(1, prefix, int(number)),
                )
            )
    return sorted(found, key=lambda citation: citation.sort)


def _fence(text: str) -> str:
    """Return a fence of backticks longer than any run of them in `text`, and of three at least."""
    longest = max((len(run) for run in re.findall(r"`+", text)), default=0)
    return "`" * (max(longest, 2) + 1)


def _permalink(reference: Reference) -> str:
    """Return the address of the lines of a file of GitHub at its commit, or "" off GitHub."""
    repo = reference.repo or ""
    if not repo.startswith("github.com/"):
        return ""
    first, _, last = (reference.lines or "").partition("-")
    anchor = ""
    if first:
        anchor = f"#L{first}-L{last}" if last else f"#L{first}"
    return f"https://{repo}/blob/{reference.rev}/{reference.path}{anchor}"


def _reference_text(reference: Reference) -> str:
    if reference.url:
        return f"### <{reference.url}>\n\n{reference.note}\n"
    place = reference.path + (f":{reference.lines}" if reference.lines else "")
    text = f"### `{place}`\n\n{reference.note}\n"
    if reference.repo:
        origin = f"{reference.repo} at {reference.rev} ({reference.license})\n"
        link = _permalink(reference)
        text += f"\n{origin}" + (f"\n<{link}>\n" if link else "")
    language = LANGUAGES.get(Path(reference.path).suffix, "")
    fence = _fence(reference.text)
    return f"{text}\n{fence}{language}\n{reference.text.removesuffix(chr(10))}\n{fence}\n"


def _topic_page(topic: Topic, cited_by: list[str]) -> str:
    text = (
        f"---\ntitle: {json.dumps(topic.title)}\nsubtitle: {json.dumps(topic.id)}\n---\n\n"
        f"# {topic.title}\n\n{topic.readme.rstrip()}\n"
    )
    if topic.references:
        text += "\n## References\n\n" + "\n".join(
            _reference_text(reference) for reference in topic.references
        )
    if cited_by:
        text += "\n## Cited by\n\n" + "".join(f"- {item}\n" for item in cited_by)
    return text


def _index_page(topics: list[Topic]) -> str:
    items = "".join(f"- [{topic.id}]({topic.id}.md): {topic.title}\n" for topic in topics)
    return (
        "---\ntitle: References\n---\n\n# References\n\n"
        "A topic collects the code and the documentation that rules and standards point at. "
        "`botica refs show <topic, rule or standard>` prints it.\n\n" + items
    )


def _check(citations: list[Citation], topics: list[Topic], docs: Path) -> None:
    known = {topic.id for topic in topics}
    for citation in citations:
        for cited in citation.topics:
            if cited not in known:
                relative = citation.page.relative_to(docs).as_posix()
                message = f"{relative} cites the topic `{cited}`, which is not in references/"
                raise ValueError(message)


def build(export: Path, docs: Path) -> None:
    """Write the topic pages of EXPORT under DOCS and list them where they are cited."""
    topics = _topics(export)
    if not topics:
        return
    citations = _citations(docs)
    _check(citations, topics, docs)
    folder = docs / "references"
    folder.mkdir(parents=True, exist_ok=True)
    for topic in topics:
        page = folder / f"{topic.id}.md"
        cited_by = [
            citation.link.replace("{page}", _link(citation.page, page))
            for citation in citations
            if topic.id in citation.topics
        ]
        page.write_text(_topic_page(topic, cited_by), encoding="utf-8")
    (folder / "index.md").write_text(_index_page(topics), encoding="utf-8")
    titles = {topic.id: topic.title for topic in topics}
    for citation in citations:
        items = "".join(
            f"- [{titles[cited]}]({_link(folder / f'{cited}.md', citation.page)})\n"
            for cited in citation.topics
        )
        text = citation.page.read_text(encoding="utf-8")
        citation.page.write_text(f"{text}\n## References\n\n{items}", encoding="utf-8")


@app.default
def main(export: Path, docs: Path) -> None:
    """Write the topic pages of EXPORT under DOCS and list them where they are cited."""
    build(export, docs)


if __name__ == "__main__":
    app()
