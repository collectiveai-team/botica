"""The modules of the script of `ci-tests-fail-on-base-python`, loaded from their templates."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from types import ModuleType

SCRIPTS = Path(__file__).resolve().parents[1] / "catalog/recipes/files/github/scripts"


def load(name: str) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f"{name}.py")
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    # The build of botica embeds every file under catalog/recipes/, so no __pycache__ is written.
    writes, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    try:
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = writes
    return module


core = load("tests_fail_on_base_core")
tfb = load("tests_fail_on_base")
recorder = load("tests_fail_on_base_pytest")
