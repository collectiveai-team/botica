//! `settings-env-schema`: the variables of `.env.schema` and the fields of the settings classes
//! of the project name the same environment.

use botica_core::env_schema::{check, Source, SETTINGS_ENV_SCHEMA};
use botica_core::{Diagnostic, Severity};

/// The findings for a schema and the Python files of a project, as `path:line:column`.
fn findings(schema: &str, files: &[(&str, &str)]) -> Vec<String> {
    run(schema, files)
        .iter()
        .map(|d| format!("{}:{}:{}", d.path, d.line, d.column))
        .collect()
}

fn run(schema: &str, files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let sources: Vec<Source<'_>> = files
        .iter()
        .map(|&(path, text)| Source { path, text })
        .collect();
    check(schema, &sources, Severity::Warning)
}

const SETTINGS: &str = "from pydantic_settings import BaseSettings\n\n\
                        class Settings(BaseSettings):\n    database_url: str\n";

#[test]
fn a_field_whose_variable_the_schema_declares_has_no_finding() {
    assert!(findings("DATABASE_URL=\n", &[("settings.py", SETTINGS)]).is_empty());
}

#[test]
fn a_field_with_no_variable_in_the_schema_is_reported_at_its_name() {
    let found = run("# @auditIgnore\nCI=\n", &[("settings.py", SETTINGS)]);

    assert_eq!(found.len(), 1);
    let finding = &found[0];
    assert_eq!(
        (finding.path.as_str(), finding.line, finding.column),
        ("settings.py", 4, 5)
    );
    assert_eq!((finding.end_line, finding.end_column), (4, 17));
    assert_eq!(finding.rule, SETTINGS_ENV_SCHEMA);
    assert_eq!(finding.severity, Severity::Warning);
    assert!(!finding.file_level);
    assert_eq!(
        finding.message,
        "the field `database_url` of `Settings` reads \
         `DATABASE_URL`, which .env.schema does not declare: add `DATABASE_URL` to .env.schema"
    );
}

#[test]
fn a_variable_that_no_settings_class_reads_is_reported_at_its_key() {
    let found = run(
        "DATABASE_URL=\n\n# @sensitive\nexport API_KEY=x\n",
        &[("settings.py", SETTINGS)],
    );

    assert_eq!(found.len(), 1);
    let finding = &found[0];
    assert_eq!(
        (finding.path.as_str(), finding.line, finding.column),
        (".env.schema", 4, 8)
    );
    assert_eq!((finding.end_line, finding.end_column), (4, 15));
    assert_eq!(
        finding.message,
        ".env.schema declares `API_KEY`, and no settings class of \
         the project reads it: add a field that reads it, remove it, or mark it \
         `# @auditIgnore` when another program reads it"
    );
}

#[test]
fn audit_ignore_above_a_variable_or_after_its_value_leaves_it_out() {
    let schema = "DATABASE_URL=\n# @auditIgnore\n# @sensitive\nTOKEN=\nCI=true # @auditIgnore\n";

    assert!(findings(schema, &[("settings.py", SETTINGS)]).is_empty());
}

#[test]
fn a_blank_line_or_the_header_divider_ends_the_decorators_of_a_variable() {
    let schema = "# @auditIgnore\n# ---\nA=\n# @auditIgnore\n\nB=\n# @auditIgnore=false\nC=\n\
                  # @auditIgnoreAll\nD=\nDATABASE_URL=\n";

    assert_eq!(
        findings(schema, &[("settings.py", SETTINGS)]),
        [
            ".env.schema:3:1",
            ".env.schema:6:1",
            ".env.schema:8:1",
            ".env.schema:10:1"
        ]
    );
}

#[test]
fn comments_and_the_lines_of_a_multiline_value_are_not_variables() {
    let schema = "# OLD=1\nDATABASE_URL=\nPEM=\"\"\"\nNOT_A_KEY=1\nNOR_THIS=2\n\"\"\"\n\
                  9LIVES=1\n=orphan\n";

    assert_eq!(
        findings(schema, &[("settings.py", SETTINGS)]),
        [".env.schema:3:1"]
    );
}

#[test]
fn the_env_prefix_goes_before_the_field_and_case_does_not_count() {
    let settings = "from pydantic_settings import BaseSettings, SettingsConfigDict\n\
                    class Settings(BaseSettings):\n    \
                    model_config = SettingsConfigDict(env_prefix=\"app_\")\n    \
                    database_url: str\n";

    assert!(findings("APP_DATABASE_URL=\n", &[("settings.py", settings)]).is_empty());
    let found = run("DATABASE_URL=\n", &[("settings.py", settings)]);
    assert_eq!(found.len(), 2);
    assert!(
        found[1].message.contains("reads `APP_DATABASE_URL`"),
        "{}",
        found[1].message
    );
}

#[test]
fn a_dict_for_model_config_and_a_dotted_base_class_count_too() {
    let settings = "import pydantic_settings\n\
                    class Settings(pydantic_settings.BaseSettings):\n    \
                    model_config = {\"env_prefix\": \"APP_\"}\n    \
                    debug: bool = False\n";

    assert!(findings("APP_DEBUG=false\n", &[("settings.py", settings)]).is_empty());
    for call in ["ConfigDict", "dict"] {
        let settings = format!(
            "class Settings(BaseSettings):\n    model_config = {call}(env_prefix=\"APP_\")\n    \
             debug: bool = False\n"
        );
        assert_eq!(
            findings("APP_DEBUG=\nX_DEBUG=\n", &[("settings.py", &settings)]),
            [".env.schema:2:1"],
            "{call}"
        );
    }
}

#[test]
fn a_case_sensitive_class_matches_the_name_exactly() {
    let settings = "class Settings(BaseSettings):\n    \
                    model_config = SettingsConfigDict(case_sensitive=True)\n    \
                    database_url: str\n";

    let found = run("DATABASE_URL=\n", &[("settings.py", settings)]);
    assert_eq!(found.len(), 2);
    assert!(
        found[1].message.contains("reads `database_url`"),
        "{}",
        found[1].message
    );
}

#[test]
fn a_plain_alias_names_the_variable_without_the_prefix() {
    let settings = "class Settings(BaseSettings):\n    \
                    model_config = SettingsConfigDict(env_prefix=\"APP_\")\n    \
                    url: str = Field(alias=\"DATABASE_URL\")\n    \
                    key: Annotated[str, Field(validation_alias=\"API_KEY\")]\n";

    assert!(findings("DATABASE_URL=\nAPI_KEY=\n", &[("settings.py", settings)]).is_empty());
}

#[test]
fn a_field_with_alias_choices_is_left_out_and_its_choices_count_as_read() {
    let settings = "class Settings(BaseSettings):\n    \
                    url: str = Field(validation_alias=AliasChoices(\"DATABASE_URL\", \"DB_URL\"))\n    \
                    item: str = Field(validation_alias=AliasPath(\"ITEMS\", 0))\n";

    assert!(findings("DB_URL=\nITEMS=\n", &[("settings.py", settings)]).is_empty());
}

#[test]
fn a_class_with_a_nested_delimiter_is_left_out_and_reads_the_nested_variables() {
    let settings = "class Settings(BaseSettings):\n    \
                    model_config = SettingsConfigDict(env_prefix=\"APP_\", env_nested_delimiter=\"__\")\n    \
                    db: Database\n    \
                    port: int = 8000\n";

    assert!(findings(
        "APP_DB__HOST=\nAPP_DB__PORT=\n",
        &[("settings.py", settings)]
    )
    .is_empty());
    assert_eq!(
        findings("APP_DBX=\n", &[("settings.py", settings)]),
        [".env.schema:1:1"]
    );
}

#[test]
fn a_parent_field_read_by_a_subclass_with_a_nested_delimiter_is_left_out() {
    let settings = "class AppBase(BaseSettings):\n    debug: bool\n\n\
                    class Settings(AppBase):\n    \
                    model_config = SettingsConfigDict(env_nested_delimiter=\"__\")\n";

    assert!(findings("", &[("settings.py", settings)]).is_empty());
}

#[test]
fn a_subclass_of_a_class_botica_cannot_list_cannot_be_listed_either() {
    let settings =
        "class AppBase(BaseSettings):\n    class Config:\n        env_prefix = \"X_\"\n\n\
                    class Settings(AppBase):\n    port: int\n";

    assert!(findings("", &[("settings.py", settings)]).is_empty());
}

#[test]
fn a_subclass_reads_the_fields_of_its_parent_with_its_own_prefix() {
    let base = "class AppBase(BaseSettings):\n    \
                model_config = SettingsConfigDict(env_prefix=\"BASE_\")\n    \
                debug: bool = False\n";
    let app = "from .base import AppBase\n\n\
               class Settings(AppBase):\n    \
               model_config = SettingsConfigDict(env_prefix=\"APP_\")\n    \
               port: int\n";

    assert!(findings(
        "APP_DEBUG=\nAPP_PORT=\n",
        &[("base.py", base), ("app.py", app)]
    )
    .is_empty());
    assert_eq!(
        findings("APP_DEBUG=\n", &[("base.py", base), ("app.py", app)]),
        ["app.py:5:5"]
    );
}

#[test]
fn a_field_missing_from_a_parent_is_shown_with_the_variable_it_reads_in_the_parent() {
    let classes = "class Settings(AppBase):\n    \
                   model_config = SettingsConfigDict(env_prefix=\"APP_\")\n\n\
                   class AppBase(BaseSettings):\n    \
                   model_config = SettingsConfigDict(env_prefix=\"BASE_\", case_sensitive=True)\n    \
                   debug: bool = False\n";

    let found = run("", &[("settings.py", classes)]);

    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .message
            .contains("the field `debug` of `AppBase` reads `BASE_debug`"),
        "{}",
        found[0].message
    );
    let found = run("app_debug=\n", &[("settings.py", classes)]);
    assert_eq!(
        found.len(),
        2,
        "the subclass is case-sensitive too: {found:?}"
    );
}

#[test]
fn a_subclass_may_set_case_sensitive_back_off() {
    let classes = "class AppBase(BaseSettings):\n    \
                   model_config = SettingsConfigDict(env_prefix=\"BASE_\", case_sensitive=True)\n    \
                   debug: bool = False\n\n\
                   class Settings(AppBase):\n    \
                   model_config = SettingsConfigDict(env_prefix=\"APP_\", case_sensitive=False)\n";

    assert!(findings("app_DEBUG=\n", &[("settings.py", classes)]).is_empty());
}

#[test]
fn a_class_whose_names_botica_cannot_list_reads_the_variables_that_end_with_its_fields() {
    for (name, settings) in [
        (
            "inner config",
            "class Settings(BaseSettings):\n    debug: bool\n    class Config:\n        env_prefix = \"X_\"\n",
        ),
        (
            "alias generator",
            "class Settings(BaseSettings):\n    \
             model_config = SettingsConfigDict(alias_generator=to_upper)\n    debug: bool\n",
        ),
        (
            "computed prefix",
            "class Settings(BaseSettings):\n    \
             model_config = SettingsConfigDict(env_prefix=PREFIX)\n    debug: bool\n",
        ),
        (
            "computed config",
            "class Settings(BaseSettings):\n    model_config = make_config()\n    debug: bool\n",
        ),
    ] {
        assert!(findings("X_DEBUG=\n", &[("settings.py", settings)]).is_empty(), "{name}");
        assert_eq!(
            findings("X_DEBUG=\nOTHER=\n", &[("settings.py", settings)]),
            [".env.schema:2:1"],
            "{name}"
        );
    }
}

#[test]
fn a_computed_alias_leaves_the_field_out() {
    let settings = "class Settings(BaseSettings):\n    \
                    url: str = Field(alias=f\"{NAME}_URL\")\n    \
                    key: str = Field(alias=ALIAS)\n    \
                    token: str = Field(alias=named(\"TOKEN\"))\n";

    assert!(findings("", &[("settings.py", settings)]).is_empty());
    assert_eq!(
        findings("TOKEN=\n", &[("settings.py", settings)]),
        [".env.schema:1:1"]
    );
}

#[test]
fn a_nested_delimiter_of_none_is_no_delimiter() {
    let settings = "class Settings(BaseSettings):\n    \
                    model_config = SettingsConfigDict(env_nested_delimiter=None)\n    \
                    port: int\n";

    assert_eq!(
        findings("", &[("settings.py", settings)]),
        ["settings.py:3:5"]
    );
}

#[test]
fn classes_that_extend_each_other_in_a_loop_end() {
    let settings = "class A(BaseSettings, B):\n    x: int\n\nclass B(A):\n    y: int\n";

    assert_eq!(
        findings("X=\nY=\n", &[("settings.py", settings)]),
        Vec::<String>::new()
    );
}

#[test]
fn a_field_that_stacks_the_settings_of_another_class_is_left_out() {
    let settings = "class DatabaseSettings(BaseSettings):\n    \
                    model_config = SettingsConfigDict(env_prefix=\"DB_\")\n    \
                    url: str\n\n\
                    class Settings(BaseSettings):\n    \
                    database: DatabaseSettings = Field(default_factory=DatabaseSettings)\n";

    assert!(findings("DB_URL=\n", &[("settings.py", settings)]).is_empty());
}

#[test]
fn class_variables_private_names_and_the_config_are_not_fields() {
    let settings = "class Settings(BaseSettings):\n    \
                    model_config: ClassVar[SettingsConfigDict] = SettingsConfigDict()\n    \
                    VERSION: ClassVar[str] = \"1\"\n    \
                    _cache: dict = {}\n    \
                    registry.default: int = 1\n    \
                    def url(self) -> str:\n        \
                    local: str = \"\"\n        \
                    return local\n";

    assert!(findings("", &[("settings.py", settings)]).is_empty());
}

#[test]
fn the_fields_of_a_class_that_is_not_a_settings_class_are_not_read() {
    let model = "class Settings(BaseModel):\n    database_url: str\n";

    assert_eq!(
        findings(
            "DATABASE_URL=\n",
            &[("settings.py", SETTINGS), ("model.py", model)]
        ),
        Vec::<String>::new()
    );
    assert!(findings("", &[("model.py", model)]).is_empty());
}

#[test]
fn a_project_with_no_settings_class_has_no_finding_about_the_schema() {
    let script = "import os\nprint(os.environ[\"TOKEN\"])\n";

    assert!(findings(
        "TOKEN=\n",
        &[("main.py", script), ("app.ts", "process.env.TOKEN")]
    )
    .is_empty());
}
