use std::fs;
use std::path::Path;

const SCHEMA: &str = "../../schema/botica.schema.json";

#[test]
fn the_published_schema_is_the_generated_one() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SCHEMA);
    let generated = botica_config::json_schema();
    if std::env::var_os("BOTICA_UPDATE_SCHEMA").is_some() {
        fs::create_dir_all(path.parent().expect("has a parent"))
            .expect("create the schema directory");
        fs::write(&path, &generated).expect("write the schema");
    }
    let published = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing; run the tests with BOTICA_UPDATE_SCHEMA=1",
            path.display()
        )
    });
    assert_eq!(
        published, generated,
        "the schema is stale; run the tests with BOTICA_UPDATE_SCHEMA=1"
    );
}

#[test]
fn the_schema_forbids_unknown_keys_and_documents_the_known_ones() {
    let schema: serde_json::Value =
        serde_json::from_str(&botica_config::json_schema()).expect("valid JSON");
    assert_eq!(
        schema["additionalProperties"],
        serde_json::Value::Bool(false)
    );
    for key in [
        "select",
        "extend-select",
        "ignore",
        "preview",
        "languages",
        "extend-exclude",
        "extend-include",
        "rule-dirs",
        "external",
        "compat-ast-grep-ignore",
        "catalogs",
        "replace",
        "rules",
        "overrides",
        "references",
        "recipes",
    ] {
        assert!(
            schema["properties"].get(key).is_some(),
            "{key} is missing from the schema"
        );
    }
}
