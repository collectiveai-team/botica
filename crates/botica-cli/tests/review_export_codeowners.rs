//! `botica review export --format codeowners`: the lines of the CODEOWNERS file, for GitHub and
//! GitLab, from the areas of `[review]`.

mod common;

use common::codeowners::{entry, github_lines, gitlab_section, printed, project, FORCED, PLATFORM};

#[test]
fn github_writes_each_glob_on_one_line_with_its_owners_and_the_forced_paths_last() {
    let dir = project(
        "github-basic",
        &format!(
            "{PLATFORM}{}",
            entry("core", &["src/**"], &["@org/core", "dev@example.com"], "")
        ),
    );

    let text = printed(&dir, &[]);

    assert_eq!(
        github_lines(&text),
        "/ci/ @org/platform\n/src/ @org/core dev@example.com\n"
    );
    let forced = text.split_once(FORCED).expect("the forced block").1;
    for line in [
        "/botica.toml @org/platform\n",
        "/.github/workflows/ @org/platform\n",
        "**/AGENTS.md @org/platform\n",
        "**/.claude/ @org/platform\n",
        "/.github/CODEOWNERS @org/platform\n",
        "/CODEOWNERS @org/platform\n",
        "/docs/CODEOWNERS @org/platform\n",
    ] {
        assert!(forced.contains(line), "{line} is missing from:\n{forced}");
    }
    assert!(text.ends_with("@org/platform\n"), "{text}");
}

const SHAPES: [&str; 12] = [
    "**",
    "a/**/*",
    "b/",
    "/c",
    "d/e",
    "f",
    "*.py",
    "g/*",
    "h/**/i/**",
    "{j,k}/**",
    "db/migr*",
    "{q,x/a}",
];

#[test]
fn each_shape_of_glob_is_written_as_github_reads_it() {
    let dir = project(
        "github-shapes",
        &format!("{PLATFORM}{}", entry("core", &SHAPES, &["@o"], "")),
    );

    let lines = github_lines(&printed(&dir, &[])).to_owned();

    assert_eq!(
        lines,
        "* @o\n**/*.py @o\n\
         **/b/ @o\n**/f @o\n/a/ @o\n/c @o\n/ci/ @org/platform\n/db/migr* @o\n/g/* @o\n/g/*/ @o\n\
         /j/ @o\n/k/ @o\n/q @o\n\
         /d/e @o\n/h/**/i/ @o\n/x/a @o\n"
    );
}

#[test]
fn each_shape_of_glob_is_written_as_gitlab_reads_it() {
    let dir = project(
        "gitlab-shapes",
        &format!("{PLATFORM}{}", entry("core", &SHAPES, &["@o"], "")),
    );

    let core = gitlab_section(&printed(&dir, &["--host", "gitlab"]), "[core][1]").to_owned();

    assert_eq!(
        core,
        "* @o\n**/*.py @o\n**/*.py/ @o\n\
         **/b/ @o\n**/f @o\n**/f/ @o\n/a/ @o\n/c @o\n/c/ @o\n/db/migr* @o\n/db/migr*/ @o\n\
         /g/* @o\n/g/*/ @o\n/j/ @o\n/k/ @o\n/q @o\n/q/ @o\n\
         /d/e @o\n/d/e/ @o\n/h/**/i/ @o\n/x/a @o\n/x/a/ @o"
    );
}

#[test]
fn gitlab_adds_the_folder_of_a_last_name_that_may_be_one() {
    let dir = project(
        "gitlab-folders",
        &format!(
            "{PLATFORM}{}",
            entry(
                "core",
                &[
                    "db/migr*",
                    "crates/*-core",
                    "**/*secret*",
                    "src/auth",
                    "g/*",
                    "x/**/"
                ],
                &["@o"],
                ""
            )
        ),
    );

    let core = gitlab_section(&printed(&dir, &["--host", "gitlab"]), "[core][1]").to_owned();

    assert_eq!(
        core,
        "**/*secret* @o\n**/*secret*/ @o\n\
         /crates/*-core @o\n/crates/*-core/ @o\n/db/migr* @o\n/db/migr*/ @o\n/g/* @o\n/g/*/ @o\n\
         /x/**/*/ @o\n\
         /src/auth @o\n/src/auth/ @o"
    );
}

#[test]
fn lines_go_from_the_lowest_tier_to_the_highest_whatever_the_order_of_the_entries() {
    let areas = [
        entry(
            "critical",
            &["src/**/auth/**"],
            &["@sec"],
            "approvals = 2\n",
        ),
        entry("core", &["src/**"], &["@core"], "tier = \"high\"\n"),
        entry("periphery", &["src/docs/**"], &["@docs"], ""),
        PLATFORM.to_owned(),
    ];
    let forward = project("order-forward", &areas.concat());
    let mut reversed_areas = areas.clone();
    reversed_areas.reverse();
    let reversed = project("order-reversed", &reversed_areas.concat());

    let text = printed(&forward, &[]);

    assert_eq!(
        github_lines(&text),
        "/src/docs/ @docs\n/ci/ @org/platform\n/src/ @core\n/src/**/auth/ @sec\n"
    );
    assert_eq!(printed(&reversed, &[]), text);
}

#[test]
fn a_file_comes_before_the_folder_of_the_same_name_and_a_trailing_slash_adds_no_depth() {
    let dir = project(
        "order-depth",
        &format!(
            "{PLATFORM}{}{}{}",
            entry("core", &["/src/", "/a/"], &["@wide"], ""),
            entry("core", &["/a/b.md"], &["@narrow"], ""),
            entry("core", &["/src"], &["@file"], "")
        ),
    );

    let text = printed(&dir, &[]);

    assert_eq!(
        github_lines(&text),
        "/a/ @wide\n/ci/ @org/platform\n/src @file\n/src/ @wide\n/a/b.md @narrow\n"
    );
}

#[test]
fn the_depth_decides_before_the_area_within_a_tier() {
    let dir = project(
        "order-area",
        &format!(
            "{}{}",
            entry("platform", &["crates/**"], &["@org/platform"], ""),
            entry("core", &["crates/app/**"], &["@org/app"], "")
        ),
    );

    let text = printed(&dir, &[]);

    assert_eq!(
        github_lines(&text),
        "/crates/ @org/platform\n/crates/app/ @org/app\n"
    );
}

#[test]
fn an_entry_with_no_owners_writes_no_line_on_either_host() {
    let dir = project(
        "no-owners",
        &format!(
            "{PLATFORM}{}{}",
            entry("core", &["src/**"], &["@core"], ""),
            entry("core", &["src/auth/**"], &[], "tier = \"high\"\n")
        ),
    );

    let github = printed(&dir, &[]);
    let gitlab = printed(&dir, &["--host", "gitlab"]);

    assert_eq!(github_lines(&github), "/ci/ @org/platform\n/src/ @core\n");
    assert_eq!(gitlab_section(&gitlab, "[core][1]"), "/src/ @core");
    assert!(!github.contains("/src/auth"), "{github}");
    assert!(!gitlab.contains("/src/auth"), "{gitlab}");
}

#[test]
fn the_forced_paths_follow_a_platform_entry_with_a_high_tier() {
    let dir = project(
        "forced-last",
        &entry(
            "platform",
            &[".github/**"],
            &["@org/release"],
            "tier = \"high\"\n",
        )
        .replace("@org/release\"]", "@org/release\", \"@org/ops\"]"),
    );

    let text = printed(&dir, &[]);

    let (entries, forced) = text.split_once(FORCED).expect("the forced block");
    assert!(
        entries.ends_with("/.github/ @org/release @org/ops\n\n"),
        "{entries}"
    );
    assert!(
        forced.contains("/.github/workflows/ @org/release @org/ops\n"),
        "{forced}"
    );
}

#[test]
fn gitlab_writes_a_section_per_area_with_its_approvals_and_periphery_as_optional() {
    let dir = project(
        "gitlab-sections",
        &[
            entry(
                "critical",
                &["db/migrations/**"],
                &["@sec"],
                "approvals = 2\n",
            ),
            entry("critical", &["/auth.py"], &["@sec"], ""),
            PLATFORM.to_owned(),
            entry("core", &["src/**"], &[], ""),
            entry("periphery", &["docs/**"], &["@docs"], ""),
        ]
        .concat(),
    );

    let text = printed(&dir, &["--host", "gitlab"]);

    let headers: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with('[') || line.starts_with("^["))
        .collect();
    assert_eq!(headers, ["^[periphery]", "[platform][1]", "[critical][2]"]);
    assert_eq!(gitlab_section(&text, "^[periphery]"), "/docs/ @docs");
    assert_eq!(
        gitlab_section(&text, "[critical][2]"),
        "/auth.py @sec\n/auth.py/ @sec\n/db/migrations/ @sec"
    );
    let platform = text
        .split_once("[platform][1]\n")
        .expect("the platform section")
        .1;
    assert!(
        platform.starts_with(&format!("/ci/ @org/platform\n{FORCED}")),
        "{platform}"
    );
    assert!(
        platform.contains("/.gitlab/CODEOWNERS @org/platform\n"),
        "{platform}"
    );
    assert!(!text.contains("/src"), "{text}");
}
