use serde_json::{Map, Value};

use super::{is_documentation, known_policy_file, DetectorEvidence, ReviewChange, SignalEvidence};

const NPM_TABLES: [&str; 5] = [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
    "overrides",
];

fn coverage_threshold(text: Option<&str>) -> Option<f64> {
    let table: toml::Table = toml::from_str(text?).ok()?;
    let value = table
        .get("tool")?
        .get("coverage")?
        .get("report")?
        .get("fail_under")?;
    value
        .as_float()
        .or_else(|| value.as_integer().map(|number| number as f64))
}

fn path_facts(change: &ReviewChange) -> Vec<SignalEvidence> {
    let mut facts = Vec::new();
    let mut add = |name: &str, detector: &str, detail: &str| {
        facts.push(SignalEvidence {
            name: name.to_owned(),
            paths: change.paths.clone(),
            detector: detector.to_owned(),
            detail: detail.to_owned(),
        });
    };
    if change.paths.iter().any(|path| {
        path.starts_with(".github/workflows/")
            || path.starts_with(".github/actions/")
            || path == ".gitlab-ci.yml"
    }) {
        add(
            "ci-changed",
            "ci-paths-v1",
            "A CI workflow or action path changed",
        );
    }
    if change
        .paths
        .iter()
        .any(|path| path.split('/').any(|part| part == "migrations"))
    {
        add(
            "migration-changed",
            "migration-paths-v1",
            "A path inside a migrations directory changed",
        );
    }
    if change.status == "D"
        && change.paths.iter().any(|path| {
            path.split('/').any(|part| part == "tests")
                || path
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name.starts_with("test_"))
        })
    {
        add(
            "tests-deleted",
            "deleted-test-paths-v1",
            "A file in a tests directory or with a test_ basename was deleted",
        );
    }
    if change
        .paths
        .iter()
        .any(|path| path.rsplit('/').next() == Some("pyproject.toml"))
    {
        if let (Some(before), Some(after)) = (
            coverage_threshold(change.before.as_deref()),
            coverage_threshold(change.after.as_deref()),
        ) {
            if after < before {
                add(
                    "coverage-lowered",
                    "coverage-pyproject-v1",
                    &format!("tool.coverage.report.fail_under decreased from {before} to {after}"),
                );
            }
        }
    }
    facts
}

fn npm(text: Option<&str>) -> Result<Map<String, Value>, String> {
    let Some(text) = text else {
        return Ok(Map::new());
    };
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or("The npm manifest is not an object")?;
    for table in NPM_TABLES {
        if object.get(table).is_some_and(|value| !value.is_object()) {
            return Err(format!("{table} is not an object"));
        }
    }
    Ok(object.clone())
}

fn npm_change(change: &ReviewChange) -> Result<Vec<String>, String> {
    let before = npm(change.before.as_deref())?;
    let after = npm(change.after.as_deref())?;
    let mut facts = Vec::new();
    let mut modified = false;
    for table in NPM_TABLES {
        let old = before.get(table).and_then(Value::as_object);
        if let Some(new) = after.get(table).and_then(Value::as_object) {
            for (name, value) in new {
                match old.and_then(|old| old.get(name)) {
                    None => facts.push(format!("{table} adds package {name}")),
                    Some(previous) if previous != value => modified = true,
                    _ => {}
                }
            }
        }
    }
    let strip = |mut object: Map<String, Value>| {
        for table in NPM_TABLES {
            object.remove(table);
        }
        object
    };
    if strip(before) != strip(after) {
        facts.push("The npm manifest changes settings outside package declarations".to_owned());
    }
    if facts.is_empty() && modified {
        return Err("Changed npm constraints require source, version and release-cooldown evidence that is not implemented".to_owned());
    }
    Ok(facts)
}

fn python_facts(change: &ReviewChange) -> Vec<SignalEvidence> {
    if change.kind != "text" || !change.paths.iter().all(|path| path.ends_with(".py")) {
        return Vec::new();
    }
    let mut signals = Vec::new();
    let old = super::python::routes(change.before.as_deref());
    let new = super::python::routes(change.after.as_deref());
    for route in new.difference(&old) {
        signals.push(SignalEvidence {
            name: "new-endpoint".to_owned(),
            paths: change.paths.clone(),
            detector: "fastapi-decorators-v1".to_owned(),
            detail: format!("A top-level FastAPI decorator adds {route}"),
        });
    }
    if super::python::assertions(change.before.as_deref())
        != super::python::assertions(change.after.as_deref())
    {
        signals.push(SignalEvidence {
            name: "assertions-changed".to_owned(),
            paths: change.paths.clone(),
            detector: "python-assert-statements-v1".to_owned(),
            detail: "The set of Python assert statements changed".to_owned(),
        });
    }
    signals
}

fn dependency_input(change: &ReviewChange) -> (&'static str, Result<Vec<String>, String>) {
    let npm = change.kind == "text"
        && change
            .paths
            .iter()
            .all(|path| path.rsplit('/').next() == Some("package.json"));
    if npm {
        ("npm-manifest-v1", npm_change(change))
    } else if change.kind == "text"
        && change
            .paths
            .iter()
            .all(|path| is_documentation(path) || known_policy_file(path))
    {
        ("documentation-and-policy-v1", Ok(Vec::new()))
    } else {
        ("dependency-and-endpoint-boundary-v1", Err("Dependency and endpoint detection is not implemented for this input; no absence of either fact is asserted".to_owned()))
    }
}

pub(super) fn detect(changes: &[ReviewChange]) -> (Vec<DetectorEvidence>, Vec<SignalEvidence>) {
    let mut detectors = Vec::new();
    let mut signals = Vec::new();
    for change in changes {
        signals.extend(path_facts(change));
        signals.extend(python_facts(change));
        let (detector, result) = dependency_input(change);
        let (status, detail) = match result {
            Ok(facts) => {
                for fact in facts {
                    signals.push(SignalEvidence {
                        name: "new-dependency".to_owned(),
                        paths: change.paths.clone(),
                        detector: detector.to_owned(),
                        detail: fact,
                    });
                }
                ("supported", "The supported detector completed".to_owned())
            }
            Err(error) => ("unsupported", error),
        };
        detectors.push(DetectorEvidence {
            detector: detector.to_owned(),
            status: status.to_owned(),
            paths: change.paths.clone(),
            detail,
        });
    }
    (detectors, signals)
}
