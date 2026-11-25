use super::{gh, matches, pull, ContextArgs};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

pub(crate) fn context(args: &ContextArgs) -> Result<Value> {
    let original = pull(&args.repository, args.pr)?;
    matches(&original, &args.base, &args.head)?;
    let (owner, name) = args
        .repository
        .split_once('/')
        .context("invalid repository")?;
    let query = "query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){pullRequest(number:$number){closingIssuesReferences(first:100){nodes{number title body url} pageInfo{hasNextPage}}}}}";
    let response = gh(&[
        "api",
        "graphql",
        "-f",
        &format!("query={query}"),
        "-f",
        &format!("owner={owner}"),
        "-f",
        &format!("name={name}"),
        "-F",
        &format!("number={}", args.pr),
    ])?;
    let linked = &response["data"]["repository"]["pullRequest"]["closingIssuesReferences"];
    ensure!(
        linked["pageInfo"]["hasNextPage"] == false && linked["nodes"].is_array(),
        "linked issue context is incomplete"
    );
    matches(&pull(&args.repository, args.pr)?, &args.base, &args.head)?;
    let description = original["body"].as_str().unwrap_or("");
    let acceptance = acceptance_criteria(description);
    Ok(
        json!({"title":original["title"],"description":original["body"],"issues":linked["nodes"],
        "acceptance_criteria":acceptance.into_iter().collect::<Vec<_>>(),
        "repository":args.repository,"pr":args.pr,"base":args.base,"head":args.head,"source":"github"}),
    )
}

/// The body of the `## Acceptance criteria` section of a description, up to the next `## `
/// heading. The heading matches in any letter case, so `## Acceptance Criteria` counts too.
/// HTML comments and empty list items are dropped, so the unfilled section of the pull request
/// template does not count as criteria.
fn acceptance_criteria(description: &str) -> Option<String> {
    const HEADING: &str = "## acceptance criteria";
    let start = description
        .char_indices()
        .map(|(index, _)| index)
        .find(|&index| {
            description
                .get(index..index + HEADING.len())
                .is_some_and(|text| text.eq_ignore_ascii_case(HEADING))
        })?;
    let section = &description[start + HEADING.len()..];
    let section = section.split("\n## ").next().unwrap_or("");
    let mut text = String::new();
    let mut rest = section;
    while let Some(open) = rest.find("<!--") {
        text.push_str(&rest[..open]);
        rest = rest[open..]
            .find("-->")
            .map_or("", |close| &rest[open + close + 3..]);
    }
    text.push_str(rest);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.trim_start_matches(['-', '*', '+']).trim().is_empty())
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

pub(crate) fn context_matches(repository: &str, pr: u64, plan: &Value) -> Result<bool> {
    let current = context(&ContextArgs {
        repository: repository.into(),
        pr,
        base: plan["inputs"]["base"]
            .as_str()
            .context("missing base")?
            .into(),
        head: plan["inputs"]["head"]
            .as_str()
            .context("missing head")?
            .into(),
    })?;
    Ok(current == plan["untrusted_context"])
}
