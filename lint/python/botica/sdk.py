"""Minimal botica extension SDK: NDJSON JSON-RPC 2.0 over stdio.

An extension registers rules with `@rule(id, languages)`; each rule receives `(path, text)` and
yields `(start_byte, end_byte, message)`. The SDK answers `initialize`, `lint/files` and
`shutdown`.
"""

import json
import sys
from collections.abc import Callable, Iterable

PROTOCOL_VERSION = 1

Finding = tuple[int, int, str]
Check = Callable[[str, str], Iterable[Finding]]

_RULES: dict[str, tuple[list[str], str, Check]] = {}


def rule(rule_id: str, languages: list[str], severity: str = "warning") -> Callable[[Check], Check]:
    def register(check: Check) -> Check:
        _RULES[rule_id] = (languages, severity, check)
        return check

    return register


def char_to_byte_offsets(text: str) -> Callable[[int], int]:
    """ast-grep-py reports character indices; the protocol carries byte offsets."""
    if text.isascii():
        return lambda index: index
    return lambda index: len(text[:index].encode())


def _language_of(path: str) -> str | None:
    return {"py": "python", "ts": "typescript", "tsx": "typescript", "js": "javascript"}.get(
        path.rsplit(".", 1)[-1]
    )


def _lint(files: list[dict[str, str]]) -> list[dict[str, object]]:
    diagnostics: list[dict[str, object]] = []
    for file in files:
        language = _language_of(file["path"])
        for rule_id, (languages, severity, check) in _RULES.items():
            if language not in languages:
                continue
            for start, end, message in check(file["path"], file["text"]):
                diagnostics.append(
                    {
                        "path": file["path"],
                        "rule": rule_id,
                        "severity": severity,
                        "start": start,
                        "end": end,
                        "message": message,
                    }
                )
    return diagnostics


def serve() -> None:
    for line in sys.stdin:
        request = json.loads(line)
        method = request["method"]
        if method == "initialize":
            result: object = {
                "protocolVersion": PROTOCOL_VERSION,
                "rules": [{"id": rid, "languages": langs} for rid, (langs, _, _) in _RULES.items()],
            }
        elif method == "lint/files":
            result = {"diagnostics": _lint(request["params"]["files"])}
        elif method == "shutdown":
            result = None
        else:
            reply = {
                "jsonrpc": "2.0",
                "id": request["id"],
                "error": {"code": -32601, "message": method},
            }
            sys.stdout.write(json.dumps(reply) + "\n")
            sys.stdout.flush()
            continue
        sys.stdout.write(
            json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}) + "\n"
        )
        sys.stdout.flush()
        if method == "shutdown":
            return
