use super::*;

#[test]
fn tests_inside_a_python_package_are_reported_and_the_ones_beside_it_are_not() {
    let dir = project("python-tests");
    backend(&dir);
    write(&dir, "backend/amarc/core/service.py", "x = 1\n");
    write(&dir, "backend/amarc/core/tests/test_service.py", "x = 1\n");
    write(&dir, "backend/amarc/core/test_inline.py", "x = 1\n");
    write(&dir, "backend/amarc/core/conftest.py", "x = 1\n");
    write(&dir, "backend/amarc/core/cusum_test.py", "x = 1\n");
    write(&dir, "backend/amarc/core/tests/factories.py", "x = 1\n");
    write(&dir, "backend/tests/test_api.py", "x = 1\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-tests-in-package"),
        [
            "backend/amarc/core/conftest.py",
            "backend/amarc/core/cusum_test.py",
            "backend/amarc/core/test_inline.py",
            "backend/amarc/core/tests/factories.py",
            "backend/amarc/core/tests/test_service.py",
        ],
        "{text}"
    );
}

#[test]
fn a_file_that_only_looks_like_a_test_is_not_one() {
    let dir = project("python-lookalikes");
    backend(&dir);
    write(&dir, "backend/amarc/attestation.py", "x = 1\n");
    write(&dir, "backend/amarc/testing.py", "x = 1\n");
    write(&dir, "backend/amarc/contest.py", "x = 1\n");
    write(&dir, "backend/amarc/latest/run.py", "x = 1\n");

    let text = lint(&dir);

    assert_eq!(reported(&text, "no-tests-in-package"), nothing(), "{text}");
}

#[test]
fn a_tests_directory_with_an_init_file_is_not_a_package_of_its_own() {
    let dir = project("tests-dir");
    write(&dir, "tests/__init__.py", "");
    write(&dir, "tests/test_a.py", "x = 1\n");
    write(&dir, "pyproject.toml", PYPROJECT);

    let text = lint(&dir);

    assert_eq!(reported(&text, "no-tests-in-package"), nothing(), "{text}");
}

#[test]
fn a_directory_that_is_not_a_package_by_convention_is_not_one_with_an_init_file() {
    let dir = project("not-packages");
    write(&dir, "pyproject.toml", PYPROJECT);
    for name in ["tests", "test", "src", "docs", "scripts", "examples"] {
        write(&dir, &format!("{name}/__init__.py"), "");
        write(&dir, &format!("{name}/test_top.py"), "x = 1\n");
    }
    write(&dir, "src/mypkg/__init__.py", "");
    write(&dir, "src/mypkg/test_a.py", "x = 1\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-tests-in-package"),
        ["src/mypkg/test_a.py"],
        "{text}"
    );
}

#[test]
fn tests_are_only_a_python_matter() {
    let dir = project("other-ecosystems-tests");
    write(&dir, "frontend/package.json", "{}\n");
    write(&dir, "frontend/src/__tests__/a.test.ts", "export {};\n");
    write(&dir, "frontend/src/b.test.ts", "export {};\n");
    write(&dir, "crates/x/Cargo.toml", "[package]\n");
    write(&dir, "crates/x/src/tests/a.rs", "fn main() {}\n");
    write(&dir, "go.mod", "module x\n");
    write(&dir, "internal/auth/auth_test.go", "package auth\n");

    let text = lint(&dir);

    assert_eq!(reported(&text, "no-tests-in-package"), nothing(), "{text}");
}

#[test]
fn a_src_layout_package_is_found_too() {
    let dir = project("src-layout");
    write(&dir, "pyproject.toml", PYPROJECT);
    write(&dir, "src/mypkg/__init__.py", "");
    write(&dir, "src/mypkg/test_a.py", "x = 1\n");
    write(&dir, "tests/test_b.py", "x = 1\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-tests-in-package"),
        ["src/mypkg/test_a.py"],
        "{text}"
    );
}
