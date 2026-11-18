use super::{gh, pull};
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub(crate) fn reviews(repository: &str, pr: u64, head: &str) -> Result<Vec<Value>> {
    pull(repository, pr)?;
    let mut latest = std::collections::BTreeMap::new();
    for page in 1..=100 {
        let response = gh(&[
            "api",
            &format!("repos/{repository}/pulls/{pr}/reviews?per_page=100&page={page}"),
        ])?;
        let reviews = response.as_array().context("missing GitHub reviews")?;
        for review in reviews {
            if review["commit_id"] != head
                || review["user"]["type"] != "User"
                || !matches!(
                    review["state"].as_str(),
                    Some("APPROVED" | "CHANGES_REQUESTED" | "DISMISSED")
                )
            {
                continue;
            }
            let reviewer = review["user"]["login"]
                .as_str()
                .context("review has no author")?;
            latest.insert(
                reviewer.to_owned(),
                json!({"reviewer":reviewer,"state":review["state"],
                "commit":head,"review_id":review["id"],"url":review["html_url"]}),
            );
        }
        if reviews.len() < 100 {
            return Ok(latest.into_values().collect());
        }
    }
    anyhow::bail!("GitHub review pagination exceeded its bound")
}
