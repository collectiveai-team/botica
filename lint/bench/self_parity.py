"""Check that the derived prefilter never drops a diagnostic.

Usage: python self_parity.py <corpus_dir> <botica_bin> [extra_rules_dir]

Runs the same binary with `--no-prefilter` and with `BOTICA_PREFILTER_AUTO=1`; the outputs must be
identical. Unlike parity.py this does not depend on the ast-grep CLI, so it also isolates prefilter
bugs from parser or grammar differences between botica and the CLI.
"""

import os
import subprocess
import sys


def lint(corpus: str, botica: str, env: dict[str, str], args: list[str]) -> set[str]:
    result = subprocess.run(
        [botica, "lint", ".", *args],
        cwd=corpus,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    return set(filter(None, result.stdout.split("\n")))


def main() -> None:
    corpus, botica = sys.argv[1], sys.argv[2]
    env = dict(os.environ)
    if len(sys.argv) > 3:
        env["BOTICA_EXTRA_RULES"] = sys.argv[3]
    plain = lint(corpus, botica, env, ["--no-prefilter"])
    derived = lint(corpus, botica, {**env, "BOTICA_PREFILTER_AUTO": "1"}, [])
    lost, extra = plain - derived, derived - plain
    sys.stdout.write(
        f"no-prefilter {len(plain)} | derived {len(derived)} | "
        f"lost by prefilter {len(lost)} | extra {len(extra)}\n"
    )
    for line in sorted(lost)[:8]:
        sys.stdout.write(f"  LOST: {line[:140]}\n")
    sys.exit(0 if plain == derived else 1)


if __name__ == "__main__":
    main()
