use botica_catalog::Catalog;
use botica_config::Config;

pub fn catalog() -> Catalog {
    Catalog::embedded().expect("the built-in catalog loads")
}

pub fn parse(text: &str) -> Config {
    Config::from_botica_toml(text).unwrap_or_else(|error| panic!("should parse:\n{error}"))
}

pub fn accepted(text: &str) -> Config {
    let config = parse(text);
    config
        .validate(&catalog())
        .unwrap_or_else(|error| panic!("should be valid:\n{error}"));
    config
}

pub fn problems(text: &str) -> String {
    match Config::from_botica_toml(text) {
        Err(error) => error.to_string(),
        Ok(config) => config
            .validate(&catalog())
            .expect_err("the configuration should be rejected")
            .to_string(),
    }
}

pub const PLATFORM: &str =
    "[[review.areas]]\nname = \"platform\"\npaths = [\"ci/**\"]\nowners = [\"@org/platform\"]\n";

pub fn with_platform(areas: &str) -> String {
    format!("{PLATFORM}\n{areas}")
}
