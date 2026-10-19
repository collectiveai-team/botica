use ast_grep_core::Matcher;
use ast_grep_language::{Language, SupportLang};

pub(crate) fn uses_key(value: &serde_json::Value, wanted: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .any(|(key, v)| (key == wanted && !v.is_null()) || uses_key(v, wanted)),
        serde_json::Value::Array(items) => items.iter().any(|v| uses_key(v, wanted)),
        _ => false,
    }
}

/// Whether an `inside` (under a `not` or not) reads the contents of an ancestor that member
/// narrowing may leave partly parsed: it uses `has` or `regex`, or a `pattern` that can match
/// such a container. In Python only top-level `class`/`def` bodies are narrowed, so a pattern
/// rooted elsewhere (e.g. `if __name__ == "__main__": …`) is safe; in JS/TS any statement ending
/// in a block may be narrowed, so every pattern counts.
pub(crate) fn inside_reads_contents(value: &serde_json::Value, language: SupportLang) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, v)| {
            let reads = key == "inside"
                && (uses_key(v, "has")
                    || uses_key(v, "regex")
                    || any_value_of(v, "pattern", &|p| !spares_members(p, language)));
            reads || inside_reads_contents(v, language)
        }),
        serde_json::Value::Array(items) => items.iter().any(|v| inside_reads_contents(v, language)),
        _ => false,
    }
}

/// Whether some value under key `wanted`, at any depth, satisfies `test`.
pub(crate) fn any_value_of(
    value: &serde_json::Value,
    wanted: &str,
    test: &dyn Fn(&serde_json::Value) -> bool,
) -> bool {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .any(|(key, v)| (key == wanted && test(v)) || any_value_of(v, wanted, test)),
        serde_json::Value::Array(items) => items.iter().any(|v| any_value_of(v, wanted, test)),
        _ => false,
    }
}

/// Whether a pattern can never match a node that member narrowing leaves partly parsed.
pub(crate) fn spares_members(pattern: &serde_json::Value, language: SupportLang) -> bool {
    const PYTHON_CONTAINERS: [&str; 4] = [
        "class_definition",
        "function_definition",
        "decorated_definition",
        "block",
    ];
    let (Some(source), SupportLang::Python) = (pattern.as_str(), language) else {
        return false;
    };
    let Some(kinds) = ast_grep_core::Pattern::try_new(source, language)
        .ok()
        .and_then(|p| p.potential_kinds())
    else {
        return false;
    };
    PYTHON_CONTAINERS
        .iter()
        .all(|kind| !kinds.contains(usize::from(language.kind_to_id(kind))))
}

/// Like `uses_key`, but not under a `not`: the prefilter takes no atom from there.
pub(crate) fn uses_key_outside_not(value: &serde_json::Value, wanted: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, v)| {
            key != "not" && ((key == wanted && !v.is_null()) || uses_key_outside_not(v, wanted))
        }),
        serde_json::Value::Array(items) => items.iter().any(|v| uses_key_outside_not(v, wanted)),
        _ => false,
    }
}

/// Whether a rule only looks at a node, its descendants and its ancestors: no siblings
/// (`follows`, `precedes`, `nthChild`), no utility rules it cannot see, no module-level match.
pub(crate) fn is_local(rule: &serde_json::Value) -> bool {
    match rule {
        serde_json::Value::Object(map) => map.iter().all(|(key, value)| match key.as_str() {
            _ if value.is_null() => true,
            "follows" | "precedes" | "nthChild" | "matches" => false,
            "kind" => value
                .as_str()
                .is_none_or(|k| k != "module" && k != "program"),
            _ => is_local(value),
        }),
        serde_json::Value::Array(items) => items.iter().all(is_local),
        _ => true,
    }
}
