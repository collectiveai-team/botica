//! `import-layers` in `botica lint`: the run reads the imports of the whole project against the
//! layers of the configuration, whatever files it is given.

mod common;

use common::{botica, project_in, stdout, write};

const LAYERS: &str = "[rules.import-layers]\nlayers = [\"app.api\", \"app.core\"]\n";

fn project(name: &str, config: &str) -> std::path::PathBuf {
    let dir = project_in("import-layers", name, Some(config));
    write(
        &dir,
        "src/app/api/routes.py",
        "from app.core import models\n",
    );
    write(&dir, "src/app/core/models.py", "import app.api.routes\n");
    write(&dir, "src/app/main.py", "x = 1\n");
    dir
}

#[test]
fn an_import_of_a_layer_above_is_an_error_whatever_file_botica_is_given() {
    let dir = project("above", LAYERS);

    let output = botica(&dir, &["src/app/main.py"]);

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains(
            "src/app/core/models.py:1:1: error [import-layers] CES-5 (import-layers): \
             `app.core.models` imports `app.api.routes`"
        ),
        "{text}"
    );
}

#[test]
fn without_layers_or_with_the_rule_ignored_there_is_no_finding() {
    let dir = project("no-layers", "");
    assert!(!stdout(&botica(&dir, &[])).contains("import-layers"));

    let dir = project(
        "ignored",
        &format!("ignore = [\"import-layers\"]\n{LAYERS}"),
    );
    assert!(!stdout(&botica(&dir, &[])).contains("import-layers"));
}

#[test]
fn the_paths_are_shown_from_where_botica_runs() {
    let dir = project("below", LAYERS);

    let text = stdout(&botica(&dir.join("src/app/api"), &["routes.py"]));

    assert!(
        text.lines()
            .any(|line| line.starts_with("../core/models.py:1:1: error [import-layers]")),
        "{text}"
    );
}

#[test]
fn layers_that_are_not_a_list_of_strings_stop_the_run() {
    let dir = project(
        "bad-layers",
        "[rules.import-layers]\nlayers = \"app.api\"\n",
    );

    let output = botica(&dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must be a list of strings"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn per_file_ignores_leave_out_the_findings_of_a_file() {
    let dir = project(
        "per-file",
        &format!(
            "{LAYERS}\n[per-file-ignores]\n\"src/app/core/models.py\" = [\"import-layers\"]\n"
        ),
    );

    assert!(!stdout(&botica(&dir, &[])).contains("import-layers"));
}
