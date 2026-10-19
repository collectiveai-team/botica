use botica_core::Severity;

#[test]
fn the_catalog_severities_map_to_the_same_named_severity_of_a_finding() {
    assert_eq!(
        Severity::from(botica_catalog::Severity::Error),
        Severity::Error
    );
    assert_eq!(
        Severity::from(botica_catalog::Severity::Warning),
        Severity::Warning
    );
    assert_eq!(
        Severity::from(botica_catalog::Severity::Info),
        Severity::Info
    );
}
