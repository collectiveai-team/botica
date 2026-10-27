//! The severity of a kind of error of pyrefly, as `[tool.pyrefly.errors]` sets it.

use toml::Value;

/// The kinds of error of pyrefly 1.4.0 whose default severity is `warn` or `info`, from
/// `ErrorKind::default_severity` in `crates/pyrefly_config/src/error_kind.rs` of the tag `1.4.0`
/// (<https://github.com/facebook/pyrefly/blob/1.4.0/crates/pyrefly_config/src/error_kind.rs>).
/// `tests::COPIED_FROM` holds the version, and its test fails when the pin of pyrefly in the root
/// `pyproject.toml`, which Dependabot moves, names another.
/// Every other kind defaults to `error` or `ignore`, and `true` makes both an error.
const DEFAULT_BELOW_ERROR: [(&str, Level); 25] = [
    ("attribute-assign-on-never", Level::Warn),
    ("coverage-missing", Level::Warn),
    ("coverage-partial", Level::Warn),
    ("deprecated", Level::Warn),
    ("direct-abstract-base-instantiation", Level::Warn),
    ("division-by-zero", Level::Warn),
    ("implicit-import", Level::Warn),
    ("invalid-decorator", Level::Warn),
    ("misplaced-ignore", Level::Warn),
    ("missing-attribute-patch-target", Level::Warn),
    ("name-mismatch", Level::Warn),
    ("non-convergent-recursion", Level::Warn),
    ("non-exhaustive-match", Level::Warn),
    ("redundant-cast", Level::Warn),
    ("redundant-condition", Level::Warn),
    ("reveal-type", Level::Info),
    ("unnecessary-comparison", Level::Warn),
    ("unnecessary-type-conversion", Level::Warn),
    ("unreachable", Level::Warn),
    ("unreachable-except-clause", Level::Warn),
    ("unreachable-match-case", Level::Warn),
    ("unresolvable-dunder-all", Level::Warn),
    ("untyped-import", Level::Warn),
    ("useless-overload-body", Level::Warn),
    ("variance-mismatch", Level::Warn),
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) enum Level {
    Ignore,
    Info,
    Warn,
    Error,
}

/// The severity that `value` gives `kind`. pyrefly reads `true` as the default severity of the
/// kind, or `error` when that default is `ignore`, and `false` as `ignore`. A kind that the table
/// does not name, as one that a newer pyrefly adds, takes `true` as `error`.
pub(super) fn severity(kind: &str, value: &Value) -> Level {
    match value {
        Value::Boolean(true) => DEFAULT_BELOW_ERROR
            .iter()
            .find(|(name, _)| *name == kind)
            .map_or(Level::Error, |(_, level)| *level),
        Value::String(name) => match name.as_str() {
            "error" => Level::Error,
            "warn" => Level::Warn,
            "info" => Level::Info,
            _ => Level::Ignore,
        },
        _ => Level::Ignore,
    }
}

#[cfg(test)]
mod tests {
    /// The version of pyrefly that `DEFAULT_BELOW_ERROR` is copied from.
    const COPIED_FROM: &str = "1.4.0";

    #[test]
    fn the_table_is_copied_from_the_pyrefly_that_the_repository_pins() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pyproject.toml");
        let text = std::fs::read_to_string(path).expect("the root pyproject.toml");
        let pyproject: toml::Table = text.parse().expect("pyproject.toml is TOML");
        let pinned = pyproject["dependency-groups"]
            .as_table()
            .expect("[dependency-groups]")
            .values()
            .filter_map(toml::Value::as_array)
            .flatten()
            .filter_map(toml::Value::as_str)
            .find_map(|spec| spec.strip_prefix("pyrefly=="))
            .expect("pyrefly==<version> in a dependency group");
        assert_eq!(
            pinned, COPIED_FROM,
            "pyproject.toml pins pyrefly {pinned}: copy the kinds whose default is warn or info from \
             ErrorKind::default_severity in crates/pyrefly_config/src/error_kind.rs of the tag \
             {pinned} of facebook/pyrefly into DEFAULT_BELOW_ERROR, then set COPIED_FROM"
        );
    }
}
