"""Generate a deterministic synthetic repo calibrated on real code.

Usage: python gen_realistic.py <out_dir> [n_files]

Unlike gen_repo.py (the same dense block in every file), this models what real repos look like:
- Python line counts are lognormal, fitted to the CPython stdlib and site-packages (median ~140,
  p90 ~900, p99 ~4 000; ~25 % of files over 400 lines); TypeScript is smaller (median ~60).
- Most code is neutral (functions, classes, loops, comprehensions, comments, docstrings).
- Each rule's construct appears in a realistic share of files, more often in bigger files, either as
  a real hit or as a decoy (a comment that mentions it) so that text-level prefilters see the same
  false positives they see in practice.
- Roughly 15 % of Python files live under tests/ (some rules ignore them).

Rates come from lint/bench/ measurements on /usr/lib/python3.14 (23.8k files) and a site-packages
(1.7k files); see .scratch/botica-lint/spike-results.md.
"""

import random
import sys
from pathlib import Path

WORDS = [
    "account",
    "batch",
    "cache",
    "client",
    "config",
    "context",
    "count",
    "cursor",
    "data",
    "event",
    "field",
    "filter",
    "group",
    "handle",
    "index",
    "item",
    "job",
    "key",
    "label",
    "limit",
    "line",
    "list",
    "loader",
    "local",
    "manager",
    "message",
    "model",
    "name",
    "node",
    "offset",
    "option",
    "order",
    "packet",
    "page",
    "parser",
    "path",
    "payload",
    "plan",
    "point",
    "queue",
    "range",
    "record",
    "reader",
    "request",
    "result",
    "route",
    "row",
    "schema",
    "segment",
    "server",
    "session",
    "setting",
    "source",
    "stack",
    "state",
    "status",
    "step",
    "store",
    "stream",
    "table",
    "target",
    "task",
    "token",
    "total",
    "update",
    "user",
    "value",
    "vector",
    "worker",
    "writer",
]

# rule -> (share of files with the construct, mean snippets when present,
#          probability that a snippet is a hit rather than a decoy)
PY_RULES = {
    "cli": (0.047, 1.2, 0.85),
    "get_logger": (0.068, 1.3, 0.80),
    "print": (0.087, 3.0, 0.80),
    "alias": (0.071, 1.2, 0.10),
    "call_return": (0.013, 1.3, 0.90),
    "literal_return": (0.094, 1.6, 0.85),
    "return_annotation": (0.049, 2.3, 0.85),
    "settings": (0.042, 2.8, 0.85),
}
TS_RULES = {"console": (0.06, 2.0, 0.80)}

PY_SNIPPETS = {
    "cli": ("args = sys.argv[1:]", "# parse these with argparse or click later"),
    "get_logger": ("log = logging.getLogger(__name__)", "# call getLogger() once per module"),
    "print": ('    print("debug", value)', "# print(value) while debugging"),
    "alias": ("Mapping = dict", "options = dict(alpha=1, beta=2)"),
    "call_return": ("def make_{n}(a):\n    return dict(a=a)", "# return dict(...) is flagged"),
    "literal_return": ('def make_{n}(a):\n    return {{"a": a}}', "# return {{ ... }} is flagged"),
    "return_annotation": (
        "def build_{n}(a) -> dict:\n    return None",
        "# -> dict[str, int] is flagged",
    ),
    "settings": ('home = os.environ["HOME"]', "# os.getenv only belongs in settings"),
}
TS_SNIPPETS = {
    "console": ('  console.log("value", value);', "// console.log(value) was removed"),
}


def lines_py(rng: random.Random) -> int:
    return min(6000, max(3, int(rng.lognormvariate(4.94, 1.456))))


def lines_ts(rng: random.Random) -> int:
    return min(3000, max(3, int(rng.lognormvariate(4.1, 0.9))))


def word(rng: random.Random) -> str:
    return rng.choice(WORDS)


def py_expr(rng: random.Random) -> str:
    a, b = word(rng), word(rng)
    if rng.random() < 0.06:
        return rng.choice([f'{{"{a}": {b}, "n": {rng.randint(1, 9)}}}', f"{{{a}, {b}}}"])
    return rng.choice(
        [
            f"{a} + {rng.randint(1, 99)}",
            f"len({a}) * {rng.randint(2, 9)}",
            f"[x for x in {a} if x]",
            f"{a}.{b}",
            f"max({a}, {b})",
            f"sorted({a}, key=lambda item: item.{b})",
            f"({a}, {b})",
            f'"%s-%s" % ({a}, {b})',
            f"{rng.random():.3f}",
        ]
    )


def py_simple(rng: random.Random, indent: str, roll: float, a: str) -> list[str] | None:
    if roll < 0.30:
        return [f"{indent}{a} = {py_expr(rng)}"]
    if roll < 0.42:
        return [f"{indent}{a} += {rng.randint(1, 9)}"]
    if roll < 0.52:
        return [f"{indent}self.{a}.append({py_expr(rng)})"]
    if roll < 0.57:
        return [f"{indent}# {a} {word(rng)} {word(rng)}"]
    if roll < 0.62:
        return [f'{indent}assert {a} is not None, "{a} missing"']
    return None


def py_compound(
    rng: random.Random, depth: int, indent: str, roll: float, a: str
) -> list[str] | None:
    inner = indent + "    "
    if roll < 0.74:
        out = [f"{indent}if {a} > {rng.randint(0, 50)}:", *py_block(rng, depth + 1, inner)]
        if rng.random() < 0.4:
            out += [f"{indent}else:", *py_block(rng, depth + 1, inner)]
        return out
    if roll < 0.84:
        return [f"{indent}for {a} in {word(rng)}s:", *py_block(rng, depth + 1, inner)]
    if roll < 0.90:
        return [
            f"{indent}try:",
            *py_block(rng, depth + 1, inner),
            f"{indent}except (ValueError, KeyError) as error:",
            f"{inner}raise RuntimeError(str(error)) from error",
        ]
    if roll < 0.94:
        return [f"{indent}with open({a}) as handle:", *py_block(rng, depth + 1, inner)]
    return None


def py_block(rng: random.Random, depth: int, indent: str) -> list[str]:
    out: list[str] = []
    for _ in range(rng.randint(2, 6)):
        roll = rng.random()
        a = word(rng)
        lines = py_simple(rng, indent, roll, a)
        if lines is None and depth < 3:
            lines = py_compound(rng, depth, indent, roll, a)
        if lines is None:
            lines = [f"{indent}{a} = {word(rng)}({py_expr(rng)}, {py_expr(rng)})"]
        out += lines
    return out


def py_neutral(rng: random.Random, n: int) -> str:
    kind = rng.random()
    name = f"{word(rng)}_{n}"
    if kind < 0.55:
        args = ", ".join(
            f"{word(rng)}: {rng.choice(['int', 'str', 'float', 'list[str]'])}"
            for _ in range(rng.randint(0, 4))
        )
        header = f"def {name}({args}) -> {rng.choice(['int', 'str', 'None', 'list[int]', 'bool'])}:"
        body = (
            [f'    """{word(rng).capitalize()} the {word(rng)}."""']
            + py_block(rng, 1, "    ")
            + ([f"    return {word(rng)}"] if rng.random() < 0.6 else [])
        )
        return "\n".join([header, *body]) + "\n"
    if kind < 0.85:
        lines = [
            f"class {name.capitalize()}:",
            f'    """{word(rng).capitalize()} {word(rng)}."""',
            "",
        ]
        for m in range(rng.randint(1, 4)):
            lines.append(f"    def method_{m}(self, {word(rng)}):")
            lines += py_block(rng, 2, "        ")
            lines.append("        return self" if rng.random() < 0.5 else "        pass")
            lines.append("")
        return "\n".join(lines)
    return f"@dataclass\nclass {name.capitalize()}:\n" + "".join(
        f"    {word(rng)}_{i}: int = {i}\n" for i in range(rng.randint(2, 8))
    )


def ts_neutral(rng: random.Random, n: int) -> str:
    name = f"{word(rng)}{n}"
    if rng.random() < 0.4:
        fields = "".join(
            f"  {word(rng)}{i}: {rng.choice(['string', 'number', 'boolean'])};\n"
            for i in range(rng.randint(2, 7))
        )
        return f"export interface {name.capitalize()} {{\n{fields}}}\n"
    body = []
    for _ in range(rng.randint(2, 6)):
        a = word(rng)
        body.append(
            rng.choice(
                [
                    f"  const {a} = items.map((item) => item.{word(rng)});",
                    f"  if ({a} > {rng.randint(0, 40)}) {{\n"
                    f"    total += {rng.randint(1, 9)};\n  }}",
                    f"  for (const {a} of {word(rng)}s) {{\n    total -= {a}.length;\n  }}",
                    f"  // {a} {word(rng)} {word(rng)}",
                    f"  let {a}: number = {rng.randint(0, 99)};",
                ]
            )
        )
    return (
        f"export function {name}(items: any[]): number {{\n  let total = 0;\n"
        + "\n".join(body)
        + "\n  return total;\n}\n"
    )


def draw_snippets(rng, rules, snippets, lines, median, n):
    """Return the snippet texts (hits and decoys) a file of `lines` lines carries."""
    out = []
    scale = (lines / median) ** 0.7
    for rule, (share, mean, hit_p) in rules.items():
        if rng.random() >= min(0.9, share * scale * 0.65):
            continue
        count = 1
        while rng.random() < 1 - 1 / max(mean, 1.0001):
            count += 1
        for _ in range(count):
            hit, decoy = snippets[rule]
            out.append((hit if rng.random() < hit_p else decoy).format(n=n))
            n += 1
    return out


def at_module_level(snippet: str, name: str) -> str:
    """Wrap an indented statement in a function so the file stays valid Python."""
    if not snippet.startswith("    "):
        return snippet
    return f"def {name}(value):\n{snippet}\n"


def next_snippet(rng: random.Random, funcs: list[str], statements: list[str], n: int) -> str | None:
    """Maybe pop one pending snippet to place after the chunk just written."""
    if not (funcs or statements) or rng.random() >= 0.3:
        return None
    if funcs and rng.random() < 0.5:
        return funcs.pop() + "\n"
    if statements:
        return at_module_level(statements.pop(), f"show_{n}")
    return None


def render_py(rng: random.Random, lines: int, index: int) -> str:
    snippets = draw_snippets(rng, PY_RULES, PY_SNIPPETS, lines, 140, index)
    header = ["import logging", "import os", "import sys", "from dataclasses import dataclass", ""]
    funcs = [s for s in snippets if s.startswith("def ")]
    statements = [s for s in snippets if not s.startswith("def ")]
    body: list[str] = []
    count = len(header)
    n = 0
    while count < lines:
        chunk = py_neutral(rng, n)
        body.append(chunk)
        count += chunk.count("\n") + 1
        n += 1
        placed = next_snippet(rng, funcs, statements, n)
        if placed is not None:
            body.append(placed)
    for s in funcs + statements:
        body.append(at_module_level(s, f"show_x{len(body)}"))
    return "\n".join(header) + "\n\n" + "\n\n".join(body)


def render_ts(rng: random.Random, lines: int, index: int) -> str:
    snippets = draw_snippets(rng, TS_RULES, TS_SNIPPETS, lines, 60, index)
    body: list[str] = []
    count = 0
    n = 0
    while count < lines:
        chunk = ts_neutral(rng, n)
        if snippets and rng.random() < 0.3 and chunk.startswith("export function"):
            chunk = chunk.replace(
                "  let total = 0;\n", "  let total = 0;\n" + snippets.pop() + "\n", 1
            )
        body.append(chunk)
        count += chunk.count("\n") + 1
        n += 1
    return "\n".join(body)


def render_tiny(rng: random.Random) -> str:
    """__init__.py / constants modules: imports and assignments only."""
    lines = [f"from .{word(rng)} import {word(rng)}" for _ in range(rng.randint(0, 6))]
    lines += [f"{word(rng).upper()} = {rng.randint(0, 99)}" for _ in range(rng.randint(0, 8))]
    return "\n".join(lines) + "\n"


def main() -> None:
    out_dir = Path(sys.argv[1])
    n_files = int(sys.argv[2]) if len(sys.argv) > 2 else 10_000
    rng = random.Random(4321)  # noqa: S311 - deterministic corpus, not cryptography
    for i in range(n_files):
        package = out_dir / f"pkg_{i // 60:03d}"
        if rng.random() < 0.7:
            if rng.random() < 0.15:
                package = package / "tests"
            package.mkdir(parents=True, exist_ok=True)
            name = "utils" if i % 997 == 0 else f"mod_{i}"
            text = render_tiny(rng) if rng.random() < 0.12 else render_py(rng, lines_py(rng), i)
            (package / f"{name}.py").write_text(text)
        else:
            package.mkdir(parents=True, exist_ok=True)
            (package / f"mod_{i}.ts").write_text(render_ts(rng, lines_ts(rng), i))


if __name__ == "__main__":
    main()
