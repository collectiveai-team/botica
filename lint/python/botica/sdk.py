"""Minimal botica extension SDK: NDJSON JSON-RPC 2.0 over stdio.

An extension registers rules with `@rule(id, languages)`. A rule of file scope receives
`(path, text, options)` and yields `(start_byte, end_byte, message)`. A rule registered with
`scope="project"` receives `(root, paths, options)`, reads the files itself, and yields
`(path, start_byte, end_byte, message)`. `options` holds the options that the configuration of
the project gives the rule. The SDK answers `initialize`, `lint/files`, `lint/project` and
`shutdown`, and runs the rules that each request names.
"""

from __future__ import annotations

import json
import sys
from collections.abc import Callable, Iterable
from dataclasses import asdict, dataclass
from typing import Any, Union

PROTOCOL_VERSION = 1

Finding = tuple[int, int, str]
ProjectFinding = tuple[str, int, int, str]
Check = Callable[..., Iterable[Union[Finding, ProjectFinding]]]


@dataclass(frozen=True)
class _Rule:
    languages: list[str]
    severity: str
    scope: str
    check: Check


_RULES: dict[str, _Rule] = {}


def rule(
    rule_id: str, languages: list[str], severity: str = "warning", scope: str = "file"
) -> Callable[[Check], Check]:
    def register(check: Check) -> Check:
        _RULES[rule_id] = _Rule(languages, severity, scope, check)
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


def _named(params: dict[str, Any], scope: str) -> list[tuple[str, _Rule, dict[str, object]]]:
    """Select the rules of `scope` that the request names, each with its options.

    A request that names no rule selects every rule of `scope`.
    """
    names = params.get("rules")
    options = params.get("options") or {}
    return [
        (rule_id, registered, options.get(rule_id) or {})
        for rule_id, registered in _RULES.items()
        if registered.scope == scope and (names is None or rule_id in names)
    ]


@dataclass(frozen=True)
class _Diagnostic:
    path: str
    rule: str
    severity: str
    start: int
    end: int
    message: str


def _lint(params: dict[str, Any]) -> list[_Diagnostic]:
    diagnostics: list[_Diagnostic] = []
    rules = _named(params, "file")
    for file in params["files"]:
        language = _language_of(file["path"])
        for rule_id, registered, options in rules:
            if language not in registered.languages:
                continue
            for start, end, message in registered.check(file["path"], file["text"], options):
                diagnostics.append(
                    _Diagnostic(file["path"], rule_id, registered.severity, start, end, message)
                )
    return diagnostics


def _lint_project(params: dict[str, Any]) -> list[_Diagnostic]:
    diagnostics: list[_Diagnostic] = []
    for rule_id, registered, options in _named(params, "project"):
        found = registered.check(params["root"], params["files"], options)
        for path, start, end, message in found:
            diagnostics.append(_Diagnostic(path, rule_id, registered.severity, start, end, message))
    return diagnostics


def serve() -> None:
    for line in sys.stdin:
        request = json.loads(line)
        method = request["method"]
        if method == "initialize":
            result: object = {
                "protocolVersion": PROTOCOL_VERSION,
                "rules": [{"id": rid, "languages": r.languages} for rid, r in _RULES.items()],
            }
        elif method == "lint/files":
            result = {"diagnostics": _lint(request["params"])}
        elif method == "lint/project":
            result = {"diagnostics": _lint_project(request["params"])}
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
            json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}, default=asdict)
            + "\n"
        )
        sys.stdout.flush()
        if method == "shutdown":
            return
