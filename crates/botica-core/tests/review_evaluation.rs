use botica_core::review::classifier::{Probabilities, Thresholds, MODEL};
use botica_core::review::evaluation::{evaluate, EvaluationCase};

fn case(number: u64, label: bool, decision: f64) -> EvaluationCase {
    EvaluationCase {
        id: format!("pr-{number}"),
        source: format!("https://github.com/owner/repo/pull/{number}"),
        base: "0".repeat(40),
        head: format!("{number:040x}"),
        label: Some(label),
        label_source: "confirmed".into(),
        label_evidence: "Synthetic worked example, not a real PR label".into(),
        floor: Some("low".into()),
        critical: Some(false),
        checks_passed: Some(true),
        reviews_passed: Some(true),
        context_complete: Some(true),
        inputs_verified: true,
        question_digest: Some(botica_core::review::questions::digest()),
        probabilities: Some(Probabilities {
            model: MODEL.into(),
            decision,
            unverifiable: 0.01,
            contained: 0.8,
            team: 0.19,
            users: 0.01,
            confidence: 0.99,
            request_id: None,
        }),
    }
}

#[test]
fn evaluation_counts_false_eligibility_without_claiming_calibration() {
    let report = evaluate(
        &[
            case(1, true, 0.01),
            case(2, false, 0.8),
            case(3, false, 0.01),
        ],
        &Thresholds::default(),
    )
    .unwrap();
    assert_eq!(report["false_eligible"], 1);
    assert_eq!(report["automation_candidates"], 2);
    assert_eq!(report["ready_for_activation"], false);
    assert_eq!(report["activation_performed"], false);
}

#[test]
fn missing_measurements_and_duplicate_change_heads_cannot_produce_a_ready_evaluation() {
    let mut pending = case(1, true, 0.01);
    pending.probabilities = None;
    let report = evaluate(&[pending], &Thresholds::default()).unwrap();
    assert_eq!(report["unmeasured"], 1);
    assert_eq!(report["ready_for_activation"], false);
    assert!(evaluate(
        &[case(1, true, 0.01), case(1, true, 0.01)],
        &Thresholds::default()
    )
    .is_err());
}

#[test]
fn technical_failures_are_unmeasured_instead_of_negative_classifier_evidence() {
    let mut cases: Vec<_> = (1..=30).map(|number| case(number, false, 0.8)).collect();
    for case in &mut cases {
        case.context_complete = Some(false);
        case.probabilities = None;
    }
    let report = evaluate(&cases, &Thresholds::default()).unwrap();
    assert_eq!(report["unmeasured"], 30);
    assert_eq!(report["evaluated"], 0);
    assert_eq!(report["relevant_negative_labels"], 0);
    assert_eq!(report["ready_for_activation"], false);
}
