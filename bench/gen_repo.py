"""Generate a deterministic synthetic repo for the botica lint measurements.

Usage: python gen_repo.py <out_dir> [n_files] [clean]

With `clean`, the blocks carry no rule hits, which separates parse cost from match cost.

70% Python, 30% TypeScript. File lengths follow a long-tailed spread (median ~110 lines, ~2%
over 400, ~0.2% over 700). A handful of catch-all modules and a mix of rule hits are planted.
"""

import random
import sys
from pathlib import Path

PY_BLOCK = """

@dataclass
class Item{n}:
    name: str
    size: int


def load_{n}(path: str) -> dict:
    data = {{"path": path, "n": {n}}}
    print("loading", path)
    return data


def summarize_{n}(items: list[Item{n}]) -> int:
    total = 0
    for item in items:
        if item.size > 10:
            total += item.size
        else:
            total -= 1
    try:
        total += 1
    except:
        pass
    return total
"""

TS_BLOCK = """
export interface Item{n} {{ name: string; size: number }}

export function load{n}(path: string): Item{n} {{
  console.log("loading", path);
  try {{
    return {{ name: path, size: {n} }};
  }} catch (error) {{
    console.log(error);
    throw error;
  }}
}}

export const sum{n} = (items: Item{n}[]): number =>
  items.reduce((acc, item) => acc + item.size, 0);
"""


def clean_py(block: str) -> str:
    return (
        block.replace("-> dict:", "-> Item{n}:")
        .replace('data = {{"path": path, "n": {n}}}', "data = Item{n}(path, {n})")
        .replace('    print("loading", path)\n', "")
        .replace("except:", "except ValueError:")
    )


def clean_ts(block: str) -> str:
    return block.replace('  console.log("loading", path);\n', "").replace(
        "    console.log(error);\n", ""
    )


def target_lines(rng: random.Random) -> int:
    roll = rng.random()
    if roll < 0.002:
        return rng.randint(700, 900)
    if roll < 0.02:
        return rng.randint(400, 690)
    return max(10, int(rng.lognormvariate(4.6, 0.55)))


def render(block: str, header: str, lines: int) -> str:
    out = [header]
    count = header.count("\n")
    n = 0
    while count < lines:
        chunk = block.format(n=n)
        out.append(chunk)
        count += chunk.count("\n")
        n += 1
    return "".join(out)


PLANTED = {
    "app/api/v1/schemas/requests/create_item.py": (
        "from pydantic import BaseModel\n\n\nclass CreateItem(BaseModel):\n    name: str\n"
    ),
    "app/core/db.py": (
        "from sqlmodel import Field, SQLModel, create_engine\n\n\n"
        "class Row(SQLModel, table=True):\n    id: int = Field(primary_key=True)\n\n\n"
        "engine = create_engine('sqlite://')\n"
    ),
    "app/config.py": (
        "from pydantic_settings import BaseSettings\n\n\n"
        "class Settings(BaseSettings):\n    debug: bool = False\n"
    ),
    "app/cli.py": "import argparse\n\nparser = argparse.ArgumentParser()\nprint('cli may print')\n",
    "app/main.py": (
        "import logging\n\nlog = logging.getLogger(__name__)\n\n\n"
        "def run() -> dict[str, int]:\n    return dict(a=1)\n\n\n"
        "if __name__ == '__main__':\n    print('entry')\n"
    ),
    "app/unicode.py": "# ñandú — non-ASCII before the hit\ndef f():\n    print('ñ')\n",
}


def plant(out_dir: Path) -> None:
    for relative, content in PLANTED.items():
        target = out_dir / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)


def main() -> None:
    out_dir = Path(sys.argv[1])
    n_files = int(sys.argv[2]) if len(sys.argv) > 2 else 10_000
    clean = len(sys.argv) > 3 and sys.argv[3] == "clean"
    py_block = clean_py(PY_BLOCK) if clean else PY_BLOCK
    ts_block = clean_ts(TS_BLOCK) if clean else TS_BLOCK
    rng = random.Random(1234)  # noqa: S311 - a deterministic corpus, not cryptography
    for i in range(n_files):
        package = out_dir / f"pkg_{i // 100:03d}"
        package.mkdir(parents=True, exist_ok=True)
        lines = target_lines(rng)
        if rng.random() < 0.7:
            name = "utils" if i % 997 == 0 else f"mod_{i}"
            header = "import logging\nfrom dataclasses import dataclass\n"
            (package / f"{name}.py").write_text(render(py_block, header, lines))
        else:
            (package / f"mod_{i}.ts").write_text(render(ts_block, "", lines))
    if not clean:
        plant(out_dir)


if __name__ == "__main__":
    main()
