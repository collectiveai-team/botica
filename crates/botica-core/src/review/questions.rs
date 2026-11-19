//! The fixed Jev question contract shared by classification and evaluation.

use anyhow::Result;
use serde_json::{json, Value};

pub fn noul(name: &str) -> Result<Value> {
    let (instructions,yes,no) = match name {
        "decision" => ("Does the change choose something the linked issue or description did not decide?",
            "Chooses user-visible behavior, a dependency, a convention or a tradeoff between alternatives not fixed by the issue.",
            "Implements a choice already fixed in the linked issue or description."),
        "unverifiable" => ("Does part of the change need direct observation beyond the supplied tests and checks?",
            "Changes user-facing text, prompts, CLI output, production settings or performance without direct verification.",
            "The supplied tests and deterministic checks verify the behavior."),
        _ => anyhow::bail!("unknown classifier question"),
    };
    Ok(json!({"type":"noul","instructions":instructions,"criteria":{"true":yes,"false":no}}))
}

pub fn all() -> Value {
    json!({
        "decision":noul("decision").expect("fixed question"),
        "unverifiable":noul("unverifiable").expect("fixed question"),
        "impact_if_wrong":{"type":"choice","instructions":"If this change is wrong despite passing its supplied checks, who notices first?",
            "criteria":{"contained":"A developer in this repository: tests, internal docs or scripts.",
                "team":"The team: CI, tooling or recipes before release.",
                "users":"A botica user: published rules, CLI or installed recipe templates."}}
    })
}

pub fn digest() -> String {
    blake3::hash(all().to_string().as_bytes())
        .to_hex()
        .to_string()
}
