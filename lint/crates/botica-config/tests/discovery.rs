use std::fs;
use std::path::{Path, PathBuf};

use botica_config::{discover, load_file};

fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("config-discovery")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("has a parent")).expect("create parents");
    fs::write(path, text).expect("write the file");
}

#[test]
fn finds_botica_toml_in_the_start_directory() {
    let root = fresh("here");
    write(&root.join("botica.toml"), "select = [\"BTSTRUC\"]\n");
    let loaded = discover(&root).expect("discovers");
    assert_eq!(loaded.root, root);
    assert_eq!(
        loaded.source.as_deref(),
        Some(root.join("botica.toml").as_path())
    );
    assert_eq!(loaded.config.select, ["BTSTRUC"]);
}

#[test]
fn walks_up_and_the_first_directory_with_a_config_is_the_root() {
    let root = fresh("up");
    write(&root.join("botica.toml"), "select = [\"all\"]\n");
    write(&root.join("pkg/botica.toml"), "select = [\"BTSTRUC\"]\n");
    let deep = root.join("pkg/src/inner");
    fs::create_dir_all(&deep).expect("create the deep directory");
    let loaded = discover(&deep).expect("discovers");
    assert_eq!(loaded.root, root.join("pkg"));
    assert_eq!(loaded.config.select, ["BTSTRUC"]);
}

#[test]
fn a_pyproject_counts_only_when_it_has_tool_botica() {
    let root = fresh("pyproject");
    write(&root.join("botica.toml"), "select = [\"all\"]\n");
    write(
        &root.join("pkg/pyproject.toml"),
        "[tool.ruff]\nline-length = 100\n",
    );
    let loaded = discover(&root.join("pkg")).expect("skips the pyproject without botica");
    assert_eq!(loaded.root, root);

    write(
        &root.join("pkg/pyproject.toml"),
        "[tool.botica]\nselect = [\"BTSTRUC\"]\n",
    );
    let loaded = discover(&root.join("pkg")).expect("uses the pyproject");
    assert_eq!(loaded.root, root.join("pkg"));
    assert_eq!(
        loaded.source.as_deref(),
        Some(root.join("pkg/pyproject.toml").as_path())
    );
}

#[test]
fn both_files_with_botica_config_in_one_directory_is_an_error() {
    let root = fresh("both");
    write(&root.join("botica.toml"), "");
    write(&root.join("pyproject.toml"), "[tool.botica]\n");
    let error = discover(&root).expect_err("ambiguous");
    let message = error.to_string();
    assert!(
        message.contains("botica.toml") && message.contains("pyproject.toml"),
        "{message}"
    );
}

#[test]
fn a_pyproject_without_botica_next_to_botica_toml_is_fine() {
    let root = fresh("benign");
    write(&root.join("botica.toml"), "");
    write(&root.join("pyproject.toml"), "[project]\nname = \"x\"\n");
    assert!(discover(&root).is_ok());
}

#[test]
fn without_config_the_defaults_apply_with_the_git_root_as_root() {
    let root = fresh("git");
    fs::create_dir_all(root.join(".git")).expect("create .git");
    let deep = root.join("a/b");
    fs::create_dir_all(&deep).expect("create the deep directory");
    let loaded = discover(&deep).expect("falls back to the defaults");
    assert_eq!(loaded.root, root);
    assert!(loaded.source.is_none());
    assert_eq!(loaded.config, botica_config::Config::default());
}

#[test]
fn a_git_worktree_marker_that_is_a_file_also_counts() {
    let root = fresh("worktree");
    write(&root.join(".git"), "gitdir: /elsewhere\n");
    let deep = root.join("a");
    fs::create_dir_all(&deep).expect("create the deep directory");
    assert_eq!(discover(&deep).expect("discovers").root, root);
}

#[test]
fn without_config_or_git_the_start_directory_is_the_root() {
    let root = std::env::temp_dir().join(format!("botica-config-nothing-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create the test directory");
    if root
        .ancestors()
        .any(|dir| dir.join(".git").exists() || dir.join("botica.toml").exists())
    {
        eprintln!("skipped: the system temp directory sits inside a repository");
        let _ = fs::remove_dir_all(&root);
        return;
    }
    let loaded = discover(&root).expect("falls back to the defaults");
    assert_eq!(loaded.root, root);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_invalid_config_found_while_walking_up_is_an_error_with_its_path() {
    let root = fresh("invalid");
    write(&root.join("pyproject.toml"), "[tool.botica\n");
    let error = discover(&root).expect_err("invalid TOML");
    assert!(error.to_string().contains("pyproject.toml"), "{error}");

    let root = fresh("unknown-key");
    write(&root.join("botica.toml"), "slect = []\n");
    let message = discover(&root).expect_err("unknown key").to_string();
    assert!(
        message.contains("botica.toml") && message.contains("slect"),
        "{message}"
    );
}

#[test]
fn an_explicit_config_wins_and_its_directory_is_the_root() {
    let root = fresh("explicit");
    write(&root.join("botica.toml"), "select = [\"all\"]\n");
    write(&root.join("other/custom.toml"), "select = [\"BTSTRUC\"]\n");
    let loaded = load_file(&root.join("other/custom.toml")).expect("loads");
    assert_eq!(loaded.root, root.join("other"));
    assert_eq!(loaded.config.select, ["BTSTRUC"]);

    write(
        &root.join("other/pyproject.toml"),
        "[tool.botica]\nselect = [\"BTTEST\"]\n",
    );
    assert_eq!(
        load_file(&root.join("other/pyproject.toml"))
            .expect("loads")
            .config
            .select,
        ["BTTEST"]
    );

    write(
        &root.join("other/empty/pyproject.toml"),
        "[project]\nname = \"x\"\n",
    );
    let error = load_file(&root.join("other/empty/pyproject.toml")).expect_err("no table");
    assert!(error.to_string().contains("tool.botica"), "{error}");

    let error = load_file(&root.join("missing.toml")).expect_err("missing file");
    assert!(error.to_string().contains("missing.toml"), "{error}");
}
