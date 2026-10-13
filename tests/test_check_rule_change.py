"""A change that adds a rule to a catalog may only touch what that kind of rule needs.

Asserted through the public seam of `scripts/check_rule_change.py`: the changes of a branch
(status and path) and the catalog on disk go in, the paths outside the contract come out (CES-65).
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "check_rule_change", Path(__file__).resolve().parents[1] / "scripts" / "check_rule_change.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
crc = importlib.util.module_from_spec(_SPEC)
sys.modules["check_rule_change"] = crc
_SPEC.loader.exec_module(crc)

BUILTIN = "lint/rules"
NATIVE_ZONE = [
    "lint/crates/botica-core/src/native.rs",
    "lint/crates/botica-core/src/layout.rs",
    "lint/crates/botica-core/src/registry.rs",
    "lint/crates/botica-core/tests/no_bar.rs",
]
INDEXES = [
    "lint/crates/botica-cli/tests/conformance.rs",
    "lint/crates/botica-config/tests/resolve_shipped_catalog.rs",
    "lint/README.md",
]


def add_rule(root: Path, catalog: str, family: str, slug: str, kind: str) -> None:
    folder = root / catalog / family / slug
    folder.mkdir(parents=True)
    (folder / "rule.toml").write_text(
        f'id = "{slug}"\n\n[[implementations]]\nkind = "{kind}"\n', encoding="utf-8"
    )


def added(path: str) -> object:
    return crc.Change("A", path)


def modified(path: str) -> object:
    return crc.Change("M", path)


def test_a_new_rule_may_only_add_its_own_folder(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    own_folder = [
        added("lint/rules/structure/no-foo/rule.toml"),
        added("lint/rules/structure/no-foo/python.yml"),
        added("lint/rules/structure/no-foo/fixtures/invalid/a.py"),
    ]

    assert crc.violations(own_folder, tmp_path) == []

    stray = [*own_folder, modified("lint/crates/botica-core/src/engine.rs")]
    assert crc.violations(stray, tmp_path) == ["lint/crates/botica-core/src/engine.rs"]


def test_a_native_rule_may_also_touch_the_native_zone_of_the_core(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-bar", "native")
    rule = added("lint/rules/structure/no-bar/rule.toml")

    assert crc.violations([rule, *map(modified, NATIVE_ZONE)], tmp_path) == []

    outside = "lint/crates/botica-cli/src/main.rs"
    assert crc.violations([rule, modified(outside)], tmp_path) == [outside]


def test_a_change_may_add_several_rules_and_each_keeps_its_own_contract(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    add_rule(tmp_path, BUILTIN, "logging", "no-qux", "native")
    changes = [
        added("lint/rules/structure/no-foo/rule.toml"),
        added("lint/rules/logging/no-qux/rule.toml"),
        added("lint/rules/logging/no-qux/fixtures/valid/a.py"),
        modified(NATIVE_ZONE[0]),
    ]

    assert crc.violations(changes, tmp_path) == []


def test_a_rule_moved_to_another_family_is_not_a_new_rule(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "fastapi", "no-foo", "ast-grep")
    moved = [
        crc.Change("D", "lint/rules/data-models/no-foo/rule.toml"),
        added("lint/rules/fastapi/no-foo/rule.toml"),
        modified("lint/README.md"),
        modified("lint/crates/botica-core/benches/lint_source.rs"),
    ]

    assert crc.violations(moved, tmp_path) == []


def test_a_rule_removed_while_another_is_added_is_still_checked(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    changes = [
        crc.Change("D", "lint/rules/structure/no-bar/rule.toml"),
        added("lint/rules/structure/no-foo/rule.toml"),
        modified("lint/crates/botica-core/src/engine.rs"),
    ]

    assert crc.violations(changes, tmp_path) == [
        "lint/crates/botica-core/src/engine.rs",
        "lint/rules/structure/no-bar/rule.toml",
    ]


def test_a_change_that_adds_no_rule_is_not_checked(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    changes = [
        modified("lint/rules/structure/no-foo/python.yml"),
        modified("lint/crates/botica-core/src/engine.rs"),
    ]

    assert crc.violations(changes, tmp_path) == []


def test_the_files_that_list_the_catalog_belong_to_the_contract_of_the_built_in_one(
    tmp_path: Path,
):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    rule = added("lint/rules/structure/no-foo/rule.toml")

    assert crc.violations([rule, *map(modified, INDEXES)], tmp_path) == []


def test_a_new_family_brings_its_family_file(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "testing", "no-foo", "ast-grep")
    changes = [
        added("lint/rules/testing/no-foo/rule.toml"),
        added("lint/rules/testing/family.toml"),
        added("lint/rules/other/family.toml"),
    ]

    assert crc.violations(changes, tmp_path) == ["lint/rules/other/family.toml"]


def test_an_extension_of_a_project_catalog_gets_its_folder_and_nothing_else(tmp_path: Path):
    project = ".botica/rules"
    add_rule(tmp_path, project, "team", "no-foo", "extension")
    rule = added(".botica/rules/team/no-foo/rule.toml")
    check = added(".botica/rules/team/no-foo/check.py")
    catalogs = (BUILTIN, project)

    assert crc.violations([rule, check], tmp_path, catalogs) == []
    assert crc.violations([rule, modified(INDEXES[0])], tmp_path, catalogs) == [INDEXES[0]]
    assert crc.violations([rule, modified(NATIVE_ZONE[0])], tmp_path, catalogs) == [NATIVE_ZONE[0]]


def test_the_changes_of_git_are_read_as_status_and_path_with_a_rename_as_two_changes():
    text = "A\tlint/rules/a/b/rule.toml\nM\tREADME.md\nD\told.rs\nA\tnew.rs\n"

    assert crc.parse_name_status(text) == [
        crc.Change("A", "lint/rules/a/b/rule.toml"),
        crc.Change("M", "README.md"),
        crc.Change("D", "old.rs"),
        crc.Change("A", "new.rs"),
    ]


def git(root: Path, *args: str) -> None:
    subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
        cwd=root,
        check=True,
        capture_output=True,
    )


def branch_that_adds_a_rule_and_edits_the_engine(root: Path) -> None:
    git(root, "init", "-q", "-b", "main")
    (root / "README.md").write_text("x\n", encoding="utf-8")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    git(root, "checkout", "-q", "-b", "feature")
    add_rule(root, BUILTIN, "structure", "no-foo", "ast-grep")
    engine = root / "lint" / "crates" / "botica-core" / "src" / "engine.rs"
    engine.parent.mkdir(parents=True)
    engine.write_text("fn main() {}\n", encoding="utf-8")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "feature")


def test_a_branch_outside_the_contract_fails_unless_it_is_acknowledged(tmp_path: Path, capsys):
    branch_that_adds_a_rule_and_edits_the_engine(tmp_path)

    assert crc.check_branch("main", tmp_path) == 1
    assert "lint/crates/botica-core/src/engine.rs" in capsys.readouterr().out

    assert crc.check_branch("main", tmp_path, acknowledged=True) == 0


def test_a_branch_that_stays_inside_the_contract_passes(tmp_path: Path):
    branch_that_adds_a_rule_and_edits_the_engine(tmp_path)
    git(tmp_path, "rm", "-q", "lint/crates/botica-core/src/engine.rs")
    git(tmp_path, "commit", "-q", "-m", "undo")

    assert crc.check_branch("main", tmp_path) == 0


def test_a_rule_that_is_not_native_gets_no_native_zone(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-baz", "ast-grep")
    rule = added("lint/rules/structure/no-baz/rule.toml")

    found = crc.violations([rule, modified(NATIVE_ZONE[0])], tmp_path)

    assert found == [NATIVE_ZONE[0]]
