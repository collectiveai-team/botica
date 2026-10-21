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

BUILTIN = "catalog/rules"
NATIVE_ZONE = [
    "crates/botica-core/src/native.rs",
    "crates/botica-core/src/layout.rs",
    "crates/botica-core/src/registry.rs",
    "crates/botica-core/tests/no_bar.rs",
]
INDEXES = [
    "crates/botica-cli/tests/conformance.rs",
    "crates/botica-config/tests/resolve_shipped_catalog.rs",
    "website/docs/guide/findings.md",
    "website/docs/extend/rules.md",
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
        added("catalog/rules/structure/no-foo/rule.toml"),
        added("catalog/rules/structure/no-foo/python.yml"),
        added("catalog/rules/structure/no-foo/fixtures/invalid/a.py"),
    ]

    assert crc.violations(own_folder, tmp_path) == []

    stray = [*own_folder, modified("crates/botica-core/src/engine.rs")]
    assert crc.violations(stray, tmp_path) == ["crates/botica-core/src/engine.rs"]


def test_a_native_rule_may_also_touch_the_native_zone_of_the_core(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-bar", "native")
    rule = added("catalog/rules/structure/no-bar/rule.toml")

    assert crc.violations([rule, *map(modified, NATIVE_ZONE)], tmp_path) == []

    outside = "crates/botica-cli/src/main.rs"
    assert crc.violations([rule, modified(outside)], tmp_path) == [outside]


def test_a_change_may_add_several_rules_and_each_keeps_its_own_contract(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    add_rule(tmp_path, BUILTIN, "logging", "no-qux", "native")
    changes = [
        added("catalog/rules/structure/no-foo/rule.toml"),
        added("catalog/rules/logging/no-qux/rule.toml"),
        added("catalog/rules/logging/no-qux/fixtures/valid/a.py"),
        modified(NATIVE_ZONE[0]),
    ]

    assert crc.violations(changes, tmp_path) == []


def test_a_rule_moved_to_another_family_is_not_a_new_rule(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "fastapi", "no-foo", "ast-grep")
    moved = [
        crc.Change("D", "catalog/rules/data-models/no-foo/rule.toml"),
        added("catalog/rules/fastapi/no-foo/rule.toml"),
        modified("website/docs/guide/lint.md"),
        modified("crates/botica-core/benches/lint_source.rs"),
    ]

    assert crc.violations(moved, tmp_path) == []


def test_a_rule_removed_while_another_is_added_is_still_checked(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    changes = [
        crc.Change("D", "catalog/rules/structure/no-bar/rule.toml"),
        added("catalog/rules/structure/no-foo/rule.toml"),
        modified("crates/botica-core/src/engine.rs"),
    ]

    assert crc.violations(changes, tmp_path) == [
        "catalog/rules/structure/no-bar/rule.toml",
        "crates/botica-core/src/engine.rs",
    ]


def test_a_change_that_adds_no_rule_is_not_checked(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    changes = [
        modified("catalog/rules/structure/no-foo/python.yml"),
        modified("crates/botica-core/src/engine.rs"),
    ]

    assert crc.violations(changes, tmp_path) == []


def test_the_files_that_list_the_catalog_belong_to_the_contract_of_the_built_in_one(
    tmp_path: Path,
):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    rule = added("catalog/rules/structure/no-foo/rule.toml")

    assert crc.violations([rule, *map(modified, INDEXES)], tmp_path) == []


def test_a_new_family_brings_its_family_file(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "testing", "no-foo", "ast-grep")
    changes = [
        added("catalog/rules/testing/no-foo/rule.toml"),
        added("catalog/rules/testing/family.toml"),
        added("catalog/rules/other/family.toml"),
    ]

    assert crc.violations(changes, tmp_path) == ["catalog/rules/other/family.toml"]


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
    text = "A\tcatalog/rules/a/b/rule.toml\nM\tREADME.md\nD\told.rs\nA\tnew.rs\n"

    assert crc.parse_name_status(text) == [
        crc.Change("A", "catalog/rules/a/b/rule.toml"),
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
    engine = root / "crates" / "botica-core" / "src" / "engine.rs"
    engine.parent.mkdir(parents=True)
    engine.write_text("fn main() {}\n", encoding="utf-8")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "feature")


def test_a_branch_outside_the_contract_fails_unless_it_is_acknowledged(tmp_path: Path, capsys):
    branch_that_adds_a_rule_and_edits_the_engine(tmp_path)

    assert crc.check_branch("main", tmp_path) == 1
    assert "crates/botica-core/src/engine.rs" in capsys.readouterr().out

    assert crc.check_branch("main", tmp_path, acknowledged=True) == 0


def test_a_branch_that_stays_inside_the_contract_passes(tmp_path: Path):
    branch_that_adds_a_rule_and_edits_the_engine(tmp_path)
    git(tmp_path, "rm", "-q", "crates/botica-core/src/engine.rs")
    git(tmp_path, "commit", "-q", "-m", "undo")

    assert crc.check_branch("main", tmp_path) == 0


def test_a_rule_that_is_not_native_gets_no_native_zone(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-baz", "ast-grep")
    rule = added("catalog/rules/structure/no-baz/rule.toml")

    found = crc.violations([rule, modified(NATIVE_ZONE[0])], tmp_path)

    assert found == [NATIVE_ZONE[0]]


def test_a_standard_is_named_by_its_set_and_its_number_whatever_its_slug_is():
    changes = [
        crc.Change("D", "standards/CES/71-keep-files-small.md"),
        added("standards/CES/71-keep-files-short.md"),
        modified("standards/CES/index.md"),
        modified("standards/index.md"),
        modified("README.md"),
    ]

    assert crc.standards_touched(changes) == ["CES/71"]


def test_the_same_number_in_two_sets_is_two_standards():
    changes = [added("standards/CES/3-a.md"), added("standards/ACME/3-b.md")]

    assert crc.standards_touched(changes) == ["ACME/3", "CES/3"]


def test_a_new_rule_may_come_with_the_standard_it_implements(tmp_path: Path):
    add_rule(tmp_path, BUILTIN, "structure", "no-foo", "ast-grep")
    changes = [
        added("catalog/rules/structure/no-foo/rule.toml"),
        added("standards/CES/71-keep-files-small.md"),
    ]

    assert crc.violations(changes, tmp_path) == []


def branch_that_touches_standards(root: Path, *paths: str) -> None:
    git(root, "init", "-q", "-b", "main")
    (root / "README.md").write_text("x\n", encoding="utf-8")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    git(root, "checkout", "-q", "-b", "feature")
    for path in paths:
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("x\n", encoding="utf-8")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "feature")


def test_a_branch_cannot_touch_two_standards_and_no_label_changes_that(tmp_path: Path, capsys):
    branch_that_touches_standards(
        tmp_path, "standards/CES/71-keep-files-small.md", "standards/CES/79-no-raw-dicts.md"
    )

    assert crc.check_branch("main", tmp_path, acknowledged=True) == 1
    out = capsys.readouterr().out
    assert "CES/71" in out
    assert "CES/79" in out


def write_standards(root: Path, *paths: str) -> None:
    for path in paths:
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("x\n", encoding="utf-8")


def test_two_standards_of_a_set_with_the_same_number_are_a_duplicate(tmp_path: Path):
    write_standards(
        tmp_path,
        "standards/CES/120-keep-files-small.md",
        "standards/CES/120-keep-files-short.md",
        "standards/CES/121-other.md",
    )

    assert crc.duplicated_standards(tmp_path) == ["CES/120"]


def test_the_same_number_in_two_sets_or_an_index_is_not_a_duplicate(tmp_path: Path):
    write_standards(
        tmp_path,
        "standards/CES/3-a.md",
        "standards/ACME/3-b.md",
        "standards/CES/index.md",
        "standards/ACME/index.md",
        "standards/index.md",
    )

    assert crc.duplicated_standards(tmp_path) == []


def test_a_branch_cannot_add_a_standard_with_a_number_that_is_taken(tmp_path: Path, capsys):
    branch_that_touches_standards(
        tmp_path, "standards/CES/120-keep-files-small.md", "standards/CES/120-keep-files-short.md"
    )

    assert crc.check_branch("main", tmp_path, acknowledged=True) == 1
    assert "CES/120" in capsys.readouterr().out


def test_a_branch_may_touch_one_standard_and_its_documentation(tmp_path: Path):
    branch_that_touches_standards(
        tmp_path, "standards/CES/71-keep-files-small.md", "standards/CES/index.md"
    )

    assert crc.check_branch("main", tmp_path) == 0
