//! `import-layers`: the modules of a layer import none of a layer above it, directly or through
//! modules of no layer, and the independent modules of one layer import none of each other.

use botica_core::import_layers::{check, Source};
use botica_core::{Diagnostic, Severity};

const LAYERS: [&str; 4] = [
    "app.entrypoints",
    "app.api",
    "app.database | app.impl",
    "app.core",
];

fn run_with(layers: &[&str], files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let layers: Vec<String> = layers.iter().map(|&layer| layer.to_owned()).collect();
    let sources: Vec<Source<'_>> = files
        .iter()
        .map(|&(path, text)| Source { path, text })
        .collect();
    check(&layers, &sources, Severity::Error)
}

fn run(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    run_with(&LAYERS, files)
}

/// The findings as `path:line:column`.
fn places(found: &[Diagnostic]) -> Vec<String> {
    found
        .iter()
        .map(|d| format!("{}:{}:{}", d.path, d.line, d.column))
        .collect()
}

const API: (&str, &str) = ("app/api/routes.py", "from app.core import models\n");
const MODELS: (&str, &str) = ("app/core/models.py", "x = 1\n");

#[test]
fn a_layer_may_import_the_layers_below_it() {
    let found = run(&[
        ("app/entrypoints/main.py", "import app.api.routes\n"),
        API,
        ("app/database/tables.py", "from app.core.models import x\n"),
        MODELS,
    ]);

    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_import_of_a_layer_above_is_reported_at_the_import() {
    let found = run(&[
        API,
        (
            "app/core/models.py",
            "import os\nfrom app.api.routes import router\n",
        ),
    ]);

    assert_eq!(places(&found), ["app/core/models.py:2:1"]);
    assert_eq!(found[0].rule, "import-layers");
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(
        found[0].message,
        "CES-5 (import-layers): `app.core.models` imports `app.api.routes`, and `app.api` is a \
         layer above `app.core`: move what it needs to `app.core` or below, or invert the \
         dependency"
    );
}

#[test]
fn the_independent_modules_of_one_layer_do_not_import_each_other() {
    let found = run(&[
        ("app/database/tables.py", "from app.impl import service\n"),
        ("app/impl/service.py", "x = 1\n"),
    ]);

    assert_eq!(places(&found), ["app/database/tables.py:1:1"]);
    assert_eq!(
        found[0].message,
        "CES-5 (import-layers): `app.database.tables` imports `app.impl.service`, and \
         `app.database` and `app.impl` are independent modules of one layer: move what both need \
         to a layer below them"
    );
}

#[test]
fn an_import_through_modules_of_no_layer_is_reported_with_the_chain() {
    let found = run(&[
        API,
        ("app/core/models.py", "from app.shared import helpers\n"),
        ("app/shared/helpers.py", "from app.shared.more import y\n"),
        ("app/shared/more.py", "import app.api.routes\n"),
    ]);

    assert_eq!(places(&found), ["app/core/models.py:1:1"]);
    assert_eq!(
        found[0].message,
        "CES-5 (import-layers): `app.core.models` reaches `app.api.routes` through \
         app.core.models -> app.shared.helpers -> app.shared.more -> app.api.routes, and `app.api` \
         is a layer above `app.core`: move what it needs to `app.core` or below, or invert the \
         dependency"
    );
}

#[test]
fn a_chain_through_a_module_of_a_layer_is_the_import_of_that_module() {
    let found = run(&[
        ("app/core/models.py", "from app.impl import service\n"),
        ("app/impl/service.py", "from app.api import routes\n"),
        ("app/api/routes.py", "x = 1\n"),
    ]);

    assert_eq!(
        places(&found),
        ["app/core/models.py:1:1", "app/impl/service.py:1:1"]
    );
    assert!(found[0].message.contains("imports `app.impl.service`"));
}

#[test]
fn relative_imports_are_read_from_the_package_of_the_module() {
    let found = run(&[
        ("app/core/__init__.py", "from ..api import routes\n"),
        (
            "app/core/models.py",
            "from . import base\nfrom ..api.routes import r\n",
        ),
        ("app/core/base.py", "x = 1\n"),
        ("app/api/routes.py", "x = 1\n"),
        ("app/api/__init__.py", ""),
    ]);

    assert_eq!(
        places(&found),
        ["app/core/__init__.py:1:1", "app/core/models.py:2:1"]
    );
}

#[test]
fn an_imported_name_that_is_no_module_is_the_module_that_holds_it() {
    let found = run(&[
        ("app/api/__init__.py", "router = 1\n"),
        ("app/core/models.py", "from app.api import router\n"),
    ]);

    assert!(found[0].message.contains("imports `app.api`,"));
}

#[test]
fn imports_inside_functions_and_type_checking_blocks_count() {
    let found = run(&[
        API,
        (
            "app/core/models.py",
            "from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    import app.api\n\n\
             def f():\n    from app.api import routes\n",
        ),
    ]);

    assert_eq!(
        places(&found),
        ["app/core/models.py:3:5", "app/core/models.py:6:5"]
    );
}

#[test]
fn the_package_is_found_under_src_and_the_tests_are_left_out() {
    let found = run(&[
        ("src/app/core/models.py", "import app.api\n"),
        ("src/app/api/__init__.py", ""),
        ("tests/app/core/test_models.py", "import app.api\n"),
    ]);

    assert_eq!(places(&found), ["src/app/core/models.py:1:1"]);
}

#[test]
fn without_layers_or_with_imports_from_outside_nothing_is_reported() {
    assert!(run_with(&[], &[("app/core/models.py", "import app.api\n")]).is_empty());
    assert!(run(&[("app/core/models.py", "import os\nimport requests.api\n")]).is_empty());
}

#[test]
fn a_statement_with_several_names_is_reported_once() {
    let found = run(&[
        ("app/core/models.py", "from app.api import a, b\n"),
        ("app/api/a.py", ""),
        ("app/api/b.py", ""),
    ]);

    assert_eq!(places(&found), ["app/core/models.py:1:1"]);
}

#[test]
fn aliases_and_star_imports_name_their_modules() {
    let found = run(&[
        ("app/api/__init__.py", ""),
        ("app/api/routes.py", ""),
        ("app/core/a.py", "import app.api.routes as r\n"),
        ("app/core/b.py", "from app.api import *\n"),
        (
            "app/core/c.py",
            "from app.shared import (\n    x as y,\n)\n",
        ),
        ("app/shared/__init__.py", "from ..api import *\n"),
    ]);

    assert_eq!(
        places(&found),
        [
            "app/core/a.py:1:1",
            "app/core/b.py:1:1",
            "app/core/c.py:1:1"
        ]
    );
    assert!(found[2]
        .message
        .contains("through app.core.c -> app.shared -> app.api"));
}

#[test]
fn a_cycle_among_modules_of_no_layer_ends_the_search() {
    let found = run(&[
        ("app/core/models.py", "import app.shared.a\n"),
        ("app/shared/a.py", "import app.shared.b\n"),
        (
            "app/shared/b.py",
            "import app.shared.a\nimport app.core.models\n",
        ),
    ]);

    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_relative_import_above_the_top_package_is_left_out() {
    assert!(run(&[("app/core/models.py", "from .... import x\n")]).is_empty());
}
