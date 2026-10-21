use super::directive::Malformed;

pub(super) fn malformed_message(reason: &Malformed) -> String {
    match reason {
        Malformed::MissingList => {
            "Malformed suppression: name the rules, as in `botica: ignore[rule-id]`".to_owned()
        }
        Malformed::EmptyList => "Malformed suppression: the list of rules is empty".to_owned(),
        Malformed::Unclosed => "Malformed suppression: the `[` is never closed".to_owned(),
        Malformed::UnknownVerb(verb) => format!(
            "Malformed suppression: `{verb}` is not a directive, use `ignore[...]` or `ignore-file[...]`"
        ),
    }
}

fn quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn unused_message(
    unused: &[&str],
    disabled: &[&str],
    file_hint: &[&str],
) -> Option<String> {
    let mut parts = Vec::new();
    if !unused.is_empty() {
        parts.push(format!("unused: {}", quoted(unused)));
    }
    if !disabled.is_empty() {
        parts.push(format!("non-enabled: {}", quoted(disabled)));
    }
    if parts.is_empty() {
        return None;
    }
    let mut message = format!("Unused suppression ({})", parts.join("; "));
    if !file_hint.is_empty() {
        message.push_str(&format!(
            "; {} reports on the whole file, use `ignore-file`",
            quoted(file_hint)
        ));
    }
    Some(message)
}

pub(super) fn judgment_on_a_line(name: &str, slug: &str) -> String {
    format!(
        "`{name}` is a judgment rule: suppress it for the whole file with \
         `botica: ignore-file[{slug}] -- <reason>`"
    )
}

pub(super) fn judgment_without_reason(name: &str) -> String {
    format!("A suppression of the judgment rule `{name}` needs a reason after `--`")
}

pub(super) fn project_scope_rule_in_a_comment(name: &str, slug: &str) -> String {
    format!(
        "`{name}` is a rule of project scope and only the configuration silences it: \
         add the path to `extend-ignores` in `[rules.{slug}]`"
    )
}
