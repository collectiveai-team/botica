//! The reason codes of the code and of the published schemas are the same set.

use std::collections::BTreeSet;

use botica_core::review::reasons::{ReasonCode, RiskReasonCode};
use serde_json::Value;

fn schema_codes(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(code) = map.get("properties").and_then(|p| p.get("code")) {
                for name in code["enum"].as_array().into_iter().flatten() {
                    found.insert(name.as_str().unwrap().to_owned());
                }
            }
            map.values().for_each(|child| schema_codes(child, found));
        }
        Value::Array(items) => items.iter().for_each(|child| schema_codes(child, found)),
        _ => {}
    }
}

#[test]
fn the_tier_schema_lists_exactly_the_risk_reason_codes() {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schema/review-tier.schema.json"),
    )
    .unwrap();
    let mut published = BTreeSet::new();
    schema_codes(&serde_json::from_str(&text).unwrap(), &mut published);
    let coded: BTreeSet<String> = RiskReasonCode::ALL
        .iter()
        .map(|code| {
            serde_json::to_value(code)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(published, coded);
}

#[test]
fn a_recorded_reason_code_reads_back_and_an_unknown_one_is_no_code() {
    for code in ["context-byte-limit", "risk-not-low", "low-confidence"] {
        let parsed = ReasonCode::parse(code).expect(code);
        assert_eq!(serde_json::to_value(parsed).unwrap(), code);
    }
    assert_eq!(ReasonCode::parse("blocked"), None);
    assert_eq!(ReasonCode::parse("Context-Limit"), None);
}
