use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-export")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the working directory");
    dir
}

fn botica(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

fn export(dir: &Path) -> Output {
    botica(
        dir,
        &[
            "rules", "export", "--format", "markdown", "--output", "site",
        ],
    )
}

fn exported(dir: &Path) -> PathBuf {
    let output = export(dir);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    dir.join("site")
}

fn page(site: &Path, relative: &str) -> String {
    fs::read_to_string(site.join(relative))
        .unwrap_or_else(|error| panic!("no page {relative}: {error}"))
}

fn readme(family: &str, slug: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../rules")
        .join(family)
        .join(slug)
        .join("README.md");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn writes_one_page_per_rule_of_the_catalog_and_an_index() {
    let dir = workdir("pages");
    let site = exported(&dir);

    let listing = botica(&dir, &["rules", "--all", "--json"]);
    let document: serde_json::Value = serde_json::from_slice(&listing.stdout).expect("JSON");
    let rules = document["rules"].as_array().expect("the rules");
    assert!(rules.len() > 20, "{} rules", rules.len());
    for rule in rules {
        let slug = rule["rule"].as_str().expect("a slug");
        let family = rule["family"].as_str().unwrap_or("botica");
        assert!(
            site.join(family).join(format!("{slug}.md")).is_file(),
            "no page for {family}/{slug}"
        );
    }

    let mut pages = 0;
    for family in fs::read_dir(&site).expect("the site").flatten() {
        if family.path().is_dir() {
            pages += fs::read_dir(family.path()).expect("a family").count();
        }
    }
    assert_eq!(pages, rules.len());
    assert!(site.join("index.md").is_file());
}

#[test]
fn a_page_is_the_front_matter_of_the_rule_followed_by_its_readme() {
    let site = exported(&workdir("page"));

    let expected = format!(
        "---\n\
         rule: \"api-schemas-extra-forbid\"\n\
         code: \"BTFAST001\"\n\
         family: \"fastapi\"\n\
         ces: \"CES-4\"\n\
         summary: \"Request and response schemas reject unknown fields\"\n\
         status: \"stable\"\n\
         since: \"0.8.0\"\n\
         default-enabled: true\n\
         default-severity: \"warning\"\n\
         languages: [\"python\"]\n\
         tags: [\"typing\"]\n\
         aliases: []\n\
         ---\n\
         \n\
         {}",
        readme("fastapi", "api-schemas-extra-forbid")
    );
    assert_eq!(page(&site, "fastapi/api-schemas-extra-forbid.md"), expected);
}

#[test]
fn a_rule_of_botica_itself_has_no_family_and_a_rule_without_a_standard_has_no_ces() {
    let site = exported(&workdir("own"));

    let expected = format!(
        "---\n\
         rule: \"io-error\"\n\
         code: \"BT001\"\n\
         summary: \"A path given on the command line cannot be read\"\n\
         status: \"stable\"\n\
         since: \"0.1.0\"\n\
         default-enabled: true\n\
         default-severity: \"error\"\n\
         languages: [\"*\"]\n\
         tags: []\n\
         aliases: []\n\
         ---\n\
         \n\
         {}",
        readme("botica", "io-error")
    );
    assert_eq!(page(&site, "botica/io-error.md"), expected);
}

#[test]
fn a_preview_rule_says_it_is_off_by_default() {
    let site = exported(&workdir("preview"));

    let text = page(&site, "architecture/settings-direct-instantiation.md");
    let front_matter: Vec<&str> = text.lines().take_while(|line| !line.is_empty()).collect();
    assert!(front_matter.contains(&"status: \"preview\""), "{text}");
    assert!(front_matter.contains(&"default-enabled: false"), "{text}");
    assert!(front_matter.contains(&"since: \"0.10.0\""), "{text}");
}

#[test]
fn the_index_groups_the_rules_by_family_in_the_order_of_their_codes() {
    let site = exported(&workdir("index"));

    let index = page(&site, "index.md");
    assert!(index.starts_with("# Rules\n\n"), "{index}");
    let section = "## fastapi\n\
                   \n\
                   Rules for FastAPI services.\n\
                   \n\
                   | Code | Rule | Status | Summary |\n\
                   |---|---|---|---|\n\
                   | `BTFAST001` | [`api-schemas-extra-forbid`](fastapi/api-schemas-extra-forbid.md) | stable | Request and response schemas reject unknown fields |\n\
                   | `BTFAST002` | [`api-schema-in-route-module`](fastapi/api-schema-in-route-module.md) | preview | Pydantic schemas are not defined in route modules |\n";
    assert!(index.contains(section), "{index}");
    let own = "## botica\n\
               \n\
               What a run reports about itself: unreadable paths and suppression comments.\n\
               \n\
               | Code | Rule | Status | Summary |\n\
               |---|---|---|---|\n\
               | `BT001` | [`io-error`](botica/io-error.md) | stable | A path given on the command line cannot be read |\n";
    assert!(index.contains(own), "{index}");
}

#[test]
fn it_describes_the_catalog_whatever_the_project_selects() {
    let dir = workdir("config");
    fs::write(
        dir.join("botica.toml"),
        "ignore = [\"no-utils\"]\n[rules.file-size-guard]\nenabled = false\n",
    )
    .expect("write the config");
    let site = exported(&dir);

    assert!(site.join("structure/no-utils.md").is_file());
    assert!(site.join("structure/file-size-guard.md").is_file());
}

#[test]
fn running_it_again_rewrites_the_pages_and_keeps_every_other_file() {
    let dir = workdir("again");
    let site = exported(&dir);
    fs::write(site.join("keep.md"), "mine\n").expect("write another file");
    fs::write(site.join("fastapi/api-schemas-extra-forbid.md"), "stale\n").expect("edit a page");

    let site = exported(&dir);

    assert_eq!(page(&site, "keep.md"), "mine\n");
    assert!(page(&site, "fastapi/api-schemas-extra-forbid.md").starts_with("---\n"));
}

#[test]
fn it_asks_for_a_known_format_and_an_output_directory() {
    let dir = workdir("arguments");

    let no_output = botica(&dir, &["rules", "export", "--format", "markdown"]);
    assert_eq!(no_output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&no_output.stderr).contains("--output"));

    let no_format = botica(&dir, &["rules", "export", "--output", "site"]);
    assert_eq!(no_format.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&no_format.stderr).contains("--format"));

    let unknown = botica(
        &dir,
        &["rules", "export", "--format", "yaml", "--output", "site"],
    );
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("markdown"));
    assert!(!dir.join("site").exists());
}

#[test]
fn it_fails_when_the_output_cannot_be_written() {
    let dir = workdir("blocked");
    fs::write(dir.join("site"), "a file, not a directory\n").expect("write a file");

    let output = export(&dir);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("botica: ") && stderr.contains("site"),
        "{stderr}"
    );
}
