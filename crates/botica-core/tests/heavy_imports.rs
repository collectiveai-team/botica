//! `no-eager-heavy-imports`: on Python 3.15 or later, a heavy library imported at the top of a
//! module without `lazy`.

use botica_core::heavy_imports::NoEagerHeavyImports;
use botica_core::{Document, NativeRule};

fn reported_with(rule: &NoEagerHeavyImports, text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    rule.check(&Document::new("app/core/model.py", text), &mut out);
    out.into_iter()
        .map(|finding| (finding.line, finding.message))
        .collect()
}

fn reported(text: &str) -> Vec<usize> {
    reported_with(&NoEagerHeavyImports::default(), text)
        .into_iter()
        .map(|(line, _)| line)
        .collect()
}

#[test]
fn an_import_at_the_top_is_reported_once_per_heavy_module() {
    assert_eq!(
        reported("import os, torch, jax.numpy\nfrom cv2 import imread\n"),
        [1, 1, 2]
    );
}

#[test]
fn the_message_gives_the_import_with_lazy_before_it() {
    let found = reported_with(
        &NoEagerHeavyImports::default(),
        "from transformers import AutoModel\n",
    );
    assert!(
        found[0]
            .1
            .contains("write `lazy from transformers import AutoModel`"),
        "{}",
        found[0].1
    );
}

#[test]
fn an_import_where_python_forbids_lazy_is_not_reported() {
    let text = "def f():\n    import torch\n\nclass A:\n    import jax\n\n\
                try:\n    import cv2\nexcept ImportError:\n    cv2 = None\n";
    assert!(reported(text).is_empty());
}

#[test]
fn an_import_in_an_if_or_a_with_at_the_top_is_reported() {
    let text = "with ctx():\n    import torch\nif FLAG:\n    import jax\n";
    assert_eq!(reported(text), [2, 4]);
}

#[test]
fn only_the_body_of_if_type_checking_is_left_out() {
    let text = "import typing\nif typing.TYPE_CHECKING:\n    import torch\nelse:\n    import jax\n";
    assert_eq!(reported(text), [5]);
}

#[test]
fn the_lazy_keyword_of_python_3_15_makes_the_import_lazy() {
    assert!(reported("lazy import torch\nlazy from transformers import AutoModel\n").is_empty());
    assert_eq!(reported("lazy = 1\nimport torch\n"), [2]);
}

#[test]
fn lazy_modules_covers_the_names_it_lists_before_the_import() {
    let text =
        "__lazy_modules__ = [\"torch\"]\nimport torch\nfrom torch import nn\nimport torch.nn\n";
    assert_eq!(reported(text), [4]);
    assert_eq!(
        reported("import torch\n__lazy_modules__ = (\"torch\",)\n"),
        [1]
    );
}

#[test]
fn a_relative_import_or_a_library_that_is_not_listed_is_not_reported() {
    assert!(reported("from . import torch\nimport numpy\nimport torchvision\n").is_empty());
}

#[test]
fn the_option_names_the_heavy_modules() {
    let rule = NoEagerHeavyImports {
        modules: vec!["sklearn.ensemble".to_owned()],
    };
    let found = reported_with(
        &rule,
        "import torch\nimport sklearn\nfrom sklearn.ensemble import X\n",
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, 3);
}
