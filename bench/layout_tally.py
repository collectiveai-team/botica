"""Reads the JSON report of `botica lint` on stdin, prints the findings of the layout rules.

Exit status 1 when the input is not a report, so a survey cannot take a failed run for zero.
"""

import json
import sys
from collections import Counter

RULES = ("no-tests-in-package", "no-notebooks", "no-artifacts-in-package")


def tally(report: str) -> str:
    """Return the number of findings of each layout rule in `report`, in the order of `RULES`."""
    counts = Counter(d["rule"] for d in json.loads(report)["diagnostics"])
    return " ".join(f"{counts[rule]:5d}" for rule in RULES)


def main() -> int:
    try:
        line = tally(sys.stdin.read())
    except (json.JSONDecodeError, KeyError):
        print("error", end="")
        return 1
    print(line, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
