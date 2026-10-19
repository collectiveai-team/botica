use super::*;

#[test]
fn rust_code_is_the_src_of_each_crate() {
    let dir = project("rust");
    write(&dir, "crates/core/Cargo.toml", "[package]\n");
    write(&dir, "crates/core/src/table.parquet", "x\n");
    write(&dir, "crates/core/tests/table.parquet", "x\n");
    write(&dir, "crates/core/benches/table.parquet", "x\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        ["crates/core/src/table.parquet"],
        "{text}"
    );
}

#[test]
fn go_code_is_internal_pkg_and_cmd() {
    let dir = project("go");
    write(&dir, "go.mod", "module x\n");
    write(&dir, "internal/auth/rows.csv", "a,b\n");
    write(&dir, "cmd/tool/rows.csv", "a,b\n");
    write(&dir, "internal/auth/testdata/rows.csv", "a,b\n");
    write(&dir, "docs/rows.csv", "a,b\n");
    write(&dir, "rows.csv", "a,b\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        ["cmd/tool/rows.csv", "internal/auth/rows.csv"],
        "{text}"
    );
}

#[test]
fn jvm_code_is_the_java_kotlin_and_scala_sources_not_the_resources() {
    let dir = project("jvm");
    write(&dir, "pom.xml", "<project/>\n");
    write(&dir, "src/main/java/com/x/rows.csv", "a,b\n");
    write(&dir, "src/main/kotlin/com/x/rows.csv", "a,b\n");
    write(&dir, "src/main/resources/rows.csv", "a,b\n");
    write(&dir, "src/test/java/com/x/rows.csv", "a,b\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        [
            "src/main/java/com/x/rows.csv",
            "src/main/kotlin/com/x/rows.csv"
        ],
        "{text}"
    );
}

#[test]
fn a_project_one_level_down_in_a_group_directory_is_found_and_dependencies_are_not() {
    let dir = project("groups");
    write(&dir, "packages/ui/package.json", "{}\n");
    write(&dir, "packages/ui/src/weights.pt", "w\n");
    write(&dir, "node_modules/dep/package.json", "{}\n");
    write(&dir, "node_modules/dep/src/weights.pt", "w\n");
    write(&dir, "examples/demo/package.json", "{}\n");
    write(&dir, "examples/demo/src/weights.pt", "w\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        ["packages/ui/src/weights.pt"],
        "{text}"
    );
}

#[test]
fn two_ecosystems_in_one_directory_are_both_looked_at() {
    let dir = project("both");
    write(&dir, "package.json", "{}\n");
    write(&dir, "pyproject.toml", PYPROJECT);
    write(&dir, "src/weights.pt", "w\n");
    write(&dir, "src/mypkg/__init__.py", "");
    write(&dir, "src/mypkg/test_a.py", "x = 1\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        ["src/weights.pt"],
        "{text}"
    );
    assert_eq!(
        reported(&text, "no-tests-in-package"),
        ["src/mypkg/test_a.py"],
        "{text}"
    );
}

#[test]
fn a_project_inside_another_project_is_not_looked_for() {
    let dir = project("nested");
    write(&dir, "backend/pyproject.toml", PYPROJECT);
    write(&dir, "backend/web/package.json", "{}\n");
    write(&dir, "backend/web/src/weights.pt", "w\n");
    write(&dir, "packages/ui/package.json", "{}\n");
    write(&dir, "packages/ui/src/weights.pt", "w\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        ["packages/ui/src/weights.pt"],
        "{text}"
    );
}
