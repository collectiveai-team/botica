use botica_catalog::Catalog;
use botica_config::{CommandLine, Config, Reason, Resolution};

fn resolve(config: &str, command_line: &CommandLine) -> Resolution {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let config: Config = toml::from_str(config).expect("config");
    let config = config
        .with_command_line(&catalog, command_line)
        .expect("valid command line");
    Resolution::new(&config, &catalog).expect("valid config")
}

fn active(resolution: &Resolution) -> Vec<&str> {
    let mut slugs: Vec<&str> = resolution
        .entries()
        .iter()
        .filter(|entry| entry.active)
        .map(|entry| entry.rule.id.as_str())
        .collect();
    slugs.sort_unstable();
    slugs
}

fn words(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn select_starts_from_scratch_whatever_the_file_selected_or_ignored() {
    let command_line = CommandLine {
        select: words(&["no-utils"]),
        ..CommandLine::default()
    };
    let resolution = resolve(
        "select = [\"file-size-guard\"]\nextend-select = [\"BT001\"]\nignore = [\"no-utils\"]\n",
        &command_line,
    );
    assert_eq!(active(&resolution), ["no-utils"]);
}

#[test]
fn ignore_adds_to_what_the_file_ignores_and_names_the_selector() {
    let command_line = CommandLine {
        ignore: words(&["BTSTRUC001"]),
        ..CommandLine::default()
    };
    let resolution = resolve("ignore = [\"io-error\"]\n", &command_line);
    let slugs = active(&resolution);
    assert!(
        !slugs.contains(&"file-size-guard") && !slugs.contains(&"io-error"),
        "{slugs:?}"
    );
    assert_eq!(
        resolution.entry("file-size-guard").expect("rule").reason,
        Reason::IgnoredBy("BTSTRUC001".to_owned())
    );
}

#[test]
fn ignore_after_select_trims_the_selection() {
    let command_line = CommandLine {
        select: words(&["BTSTRUC"]),
        ignore: words(&["no-utils"]),
        ..CommandLine::default()
    };
    let resolution = resolve("", &command_line);
    assert_eq!(active(&resolution), ["file-size-guard", "repo-shape"]);
}

#[test]
fn nothing_given_leaves_the_file_as_it_was() {
    let resolution = resolve("select = [\"no-utils\"]\n", &CommandLine::default());
    assert_eq!(active(&resolution), ["no-utils"]);
}

#[test]
fn a_selector_that_names_nothing_is_an_error_that_says_which_option() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let command_line = CommandLine {
        select: words(&["no-such-rule"]),
        ignore: words(&["BTNOPE001"]),
        ..CommandLine::default()
    };
    let error = Config::default()
        .with_command_line(&catalog, &command_line)
        .expect_err("unknown selectors");
    let message = error.to_string();
    assert!(
        message.contains("--select") && message.contains("no-such-rule"),
        "{message}"
    );
    assert!(
        message.contains("--ignore") && message.contains("BTNOPE001"),
        "{message}"
    );
}

#[test]
fn preview_turns_on_the_opt_in_for_preview_rules() {
    let command_line = CommandLine {
        preview: Some(true),
        ..CommandLine::default()
    };
    let config = Config::default()
        .with_command_line(
            &Catalog::embedded().expect("embedded catalog"),
            &command_line,
        )
        .expect("valid");
    assert!(config.preview);
}

#[test]
fn no_preview_turns_off_what_the_file_turned_on_and_nothing_given_leaves_it() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let on: Config = toml::from_str("preview = true\n").expect("config");
    let off = CommandLine {
        preview: Some(false),
        ..CommandLine::default()
    };
    assert!(
        !on.clone()
            .with_command_line(&catalog, &off)
            .expect("valid")
            .preview
    );
    assert!(
        on.with_command_line(&catalog, &CommandLine::default())
            .expect("valid")
            .preview
    );
}

#[test]
fn extend_select_adds_to_the_selection_of_the_file() {
    let command_line = CommandLine {
        extend_select: words(&["BT001"]),
        ..CommandLine::default()
    };
    let resolution = resolve(
        "select = [\"no-utils\"]\nextend-select = [\"BT003\"]\n",
        &command_line,
    );
    assert_eq!(
        active(&resolution),
        ["invalid-suppression", "io-error", "no-utils"]
    );
}

#[test]
fn extend_select_after_select_adds_to_the_fresh_selection() {
    let command_line = CommandLine {
        select: words(&["no-utils"]),
        extend_select: words(&["BT001"]),
        ..CommandLine::default()
    };
    let resolution = resolve("extend-select = [\"BT003\"]\n", &command_line);
    assert_eq!(active(&resolution), ["io-error", "no-utils"]);
}

#[test]
fn an_extend_select_that_names_nothing_is_an_error_that_says_which_option() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let command_line = CommandLine {
        extend_select: words(&["no-such-rule"]),
        ..CommandLine::default()
    };
    let message = Config::default()
        .with_command_line(&catalog, &command_line)
        .expect_err("unknown selector")
        .to_string();
    assert!(
        message.contains("--extend-select") && message.contains("no-such-rule"),
        "{message}"
    );
}
