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
    let acceptance = description
        .split_once("## Acceptance criteria")
        .map(|(_, section)| section.split("\n## ").next().unwrap_or("").trim())
        .filter(|section| !section.is_empty());
    Ok(
        json!({"title":original["title"],"description":original["body"],"issues":linked["nodes"],
        "acceptance_criteria":acceptance.into_iter().collect::<Vec<_>>(),
        "repository":args.repository,"pr":args.pr,"base":args.base,"head":args.head,"source":"github"}),
    )
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
