"""Write a stress copy of a corpus with comments injected between tokens.

Usage: python mutate_comments.py <src_dir> <dst_dir>

Python: after `(`, `[` and `,`, half the time, insert `  # c<N>` and a newline (legal inside
brackets). JS/TS: after `(`, `=`, `.`, `,` and `:` insert a block or line comment, and a block
comment before some `)`. Strings get mangled too: the output need not be valid code, it only has to
be the same input for both tools. ast-grep 0.45 skips comments in `smart` mode and 0.41-0.44 do not,
so this exercises the derived prefilter where the versions differ.
"""

import random
import re
import shutil
import sys
from pathlib import Path


def mutate_py(text: str, rng: random.Random) -> str:
    def insert(match: re.Match[str]) -> str:
        comment = f"  # c{rng.randint(0, 9)}\n" if rng.random() < 0.5 else ""
        return match.group(0) + comment

    return re.sub(r"[(\[,]", insert, text)


def mutate_js(text: str, rng: random.Random) -> str:
    def insert(match: re.Match[str]) -> str:
        roll = rng.random()
        if roll < 0.35:
            return match.group(0) + " /* c */ "
        if roll < 0.45:
            return match.group(0) + " // c\n"
        return match.group(0)

    text = re.sub(r"[(=.,:]", insert, text)
    return re.sub(r"\)", lambda m: (" /* c */" if rng.random() < 0.3 else "") + m.group(0), text)


def main() -> None:
    src, dst = Path(sys.argv[1]), Path(sys.argv[2])
    rng = random.Random(7)  # noqa: S311 - deterministic corpus, not cryptography
    shutil.rmtree(dst, ignore_errors=True)
    written = 0
    for path in sorted(src.rglob("*")):
        if path.suffix in {".py", ".pyi"}:
            mutate = mutate_py
        elif path.suffix in {".js", ".jsx", ".ts", ".tsx"}:
            mutate = mutate_js
        else:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        target = dst / path.relative_to(src)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(mutate(text, rng), encoding="utf-8")
        written += 1
    sys.stdout.write(f"{written} files written to {dst}\n")


if __name__ == "__main__":
    main()
