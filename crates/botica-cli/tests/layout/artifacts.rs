use super::*;
use common::write_bytes;

#[test]
fn code_in_a_directory_named_like_an_artifact_is_not_one_but_data_is() {
    let dir = project("artifact-names");
    backend(&dir);
    write(&dir, "backend/amarc/data/__init__.py", "");
    write(&dir, "backend/amarc/data/loader.py", "x = 1\n");
    write(&dir, "backend/amarc/data/rows.csv", "a,b\n");
    write(&dir, "backend/amarc/reports/__init__.py", "");
    write(&dir, "backend/amarc/reports/render.py", "x = 1\n");
    write(&dir, "backend/amarc/resources/schema.yaml", "a: 1\n");
    write(&dir, "backend/amarc/resources/py.typed", "");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        [
            "backend/amarc/data/rows.csv",
            "backend/amarc/resources/schema.yaml"
        ],
        "{text}"
    );
}

#[test]
fn a_binary_data_file_is_reported_though_the_linter_never_reads_it() {
    let dir = project("artifact-binary");
    backend(&dir);
    write_bytes(
        &dir,
        "backend/amarc/weights.pt",
        b"\x80\x02\x00\x00PK\x03\x04\x00\x00",
    );
    write_bytes(&dir, "backend/amarc/table.parquet", b"PAR1\x00\x00\x00\x00");
    write_bytes(&dir, "backend/weights.pt", b"\x00\x01\x02");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        ["backend/amarc/table.parquet", "backend/amarc/weights.pt"],
        "{text}"
    );
}

#[test]
fn data_and_resources_beside_the_package_are_where_they_belong() {
    let dir = project("artifacts-beside");
    backend(&dir);
    write(&dir, "backend/data/rows.csv", "a,b\n");
    write(&dir, "backend/resources/schema.yaml", "a: 1\n");
    write(&dir, "backend/reports/q1.txt", "r\n");
    write(&dir, "backend/models/weights.pt", "w\n");
    write(&dir, "data/rows.parquet", "a\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        nothing(),
        "{text}"
    );
}

#[test]
fn data_files_are_known_by_their_extension_not_by_the_directory_they_are_in() {
    let dir = project("artifact-extensions");
    backend(&dir);
    for file in [
        "a.csv",
        "a.parquet",
        "a.pkl",
        "a.pt",
        "a.h5",
        "a.npy",
        "a.onnx",
        "a.db",
        "a.gz",
        "A.CSV",
    ] {
        write(&dir, &format!("backend/amarc/{file}"), "x\n");
    }
    for file in ["a.py", "a.yaml", "a.json", "a.txt", "a.md", "a.csv.py"] {
        write(&dir, &format!("backend/amarc/{file}"), "x\n");
    }

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        [
            "backend/amarc/A.CSV",
            "backend/amarc/a.csv",
            "backend/amarc/a.db",
            "backend/amarc/a.gz",
            "backend/amarc/a.h5",
            "backend/amarc/a.npy",
            "backend/amarc/a.onnx",
            "backend/amarc/a.parquet",
            "backend/amarc/a.pkl",
            "backend/amarc/a.pt",
        ],
        "{text}"
    );
}

#[test]
fn what_tests_and_documentation_carry_is_left_alone() {
    let dir = project("artifact-exempt");
    backend(&dir);
    for place in [
        "tests",
        "fixtures",
        "testdata",
        "examples",
        "docs",
        "snapshots",
        "api_tests",
        "e2e-tests",
    ] {
        write(&dir, &format!("backend/amarc/{place}/rows.csv"), "a,b\n");
    }

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        nothing(),
        "{text}"
    );
}

#[test]
fn in_the_other_ecosystems_only_the_kind_of_file_counts() {
    let dir = project("artifacts-js");
    write(&dir, "frontend/package.json", "{}\n");
    write(&dir, "frontend/src/models/weights.pt", "w\n");
    write(&dir, "frontend/src/data/list.json", "[]\n");
    write(&dir, "frontend/src/resources/strings.yaml", "a: 1\n");
    write(&dir, "frontend/src/assets/logo.svg", "<svg/>\n");
    write(&dir, "frontend/public/rows.csv", "a,b\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        ["frontend/src/models/weights.pt"],
        "{text}"
    );
}

#[test]
fn a_binary_data_file_is_reported_however_it_is_reached() {
    let dir = project("artifact-binary-named");
    backend(&dir);
    write_bytes(
        &dir,
        "backend/amarc/weights.pt",
        b"\x80\x02\x00\x00PK\x03\x04\x00\x00",
    );
    let cache = dir.join("cache");
    let cache = cache.to_str().expect("UTF-8 path");

    for args in [
        &["lint", "backend/amarc/weights.pt", "--no-astgrep"][..],
        &["lint", ".", "--no-astgrep", "--cache-dir", cache],
        &["lint", ".", "--no-astgrep", "--cache-dir", cache],
    ] {
        let text = stdout(&command(&dir, args));

        assert_eq!(
            reported(&text, "no-artifacts-in-package"),
            ["backend/amarc/weights.pt"],
            "{args:?}:\n{text}"
        );
    }
}
