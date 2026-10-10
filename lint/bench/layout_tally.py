"""Reads the JSON report of `botica lint` on stdin, prints the findings of the layout rules."""

import json
import sys
from collections import Counter

RULES = ("no-tests-in-package", "no-notebooks", "no-artifacts-in-package")

try:
    counts = Counter(d["rule"] for d in json.load(sys.stdin)["diagnostics"])
except (json.JSONDecodeError, KeyError):
    print("error", end="")
else:
    print(" ".join(f"{counts[rule]:5d}" for rule in RULES), end="")
