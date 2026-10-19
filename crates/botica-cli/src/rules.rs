use anyhow::Result;
use botica_astgrep::{AstGrepRule, AstGrepSet, Tuning};
use botica_catalog::Kind;
use botica_config::Resolution;
use botica_core::suppress::Policy;
use botica_core::{Engine, Exclusion, NativePlanner, NativeRule};
use std::path::Path;

/// An active rule of the catalog that is implemented with ast-grep: its YAML and the regex that
/// `rule.toml` declares for the prefilter, if any.
struct Source<'r> {
    yaml: &'r str,
    prefilter: Option<&'r str>,
}

fn catalog_sources(resolution: &Resolution) -> Vec<Source<'_>> {
    resolution
        .entries()
        .iter()
        .filter(|entry| entry.active)
        .flat_map(|entry| {
            entry
                .rule
                .implementations
                .iter()
                .filter(|implementation| implementation.kind == Kind::AstGrep)
                .filter_map(|implementation| implementation.source.as_deref())
                .map(|yaml| Source {
                    yaml,
                    prefilter: entry.rule.prefilter.as_deref(),
                })
        })
        .collect()
}

pub struct EngineOptions<'a> {
    pub astgrep: bool,
    pub resolution: &'a Resolution,
    pub tuning: Tuning,
    /// Extra ast-grep YAML rules (`*.yml` in this directory), for parity checks on more languages.
    pub extra_rules: Option<&'a Path>,
    /// Time each rule (`--timings`).
    pub timings: bool,
}

pub fn build_engine(
    options: &EngineOptions<'_>,
    native: Vec<Box<dyn NativeRule>>,
    planner: NativePlanner,
    policy: Policy,
    exclusion: Option<Exclusion>,
) -> Result<Engine> {
    let mut rules = Vec::new();
    if options.astgrep {
        for source in catalog_sources(options.resolution) {
            for mut rule in AstGrepRule::from_yaml(source.yaml)? {
                if let Some(pattern) = source.prefilter {
                    rule.declare_prefilter(pattern);
                }
                rules.push(rule);
            }
        }
        for yaml in extra_rule_texts(options.extra_rules)? {
            rules.extend(AstGrepRule::from_yaml(&yaml)?);
        }
    }
    let policy = policy.with_astgrep_rules(rules.iter().map(AstGrepRule::id));
    let set = AstGrepSet::with_tuning(rules, options.tuning);
    let engine = Engine::new(native, set)
        .with_planner(planner)
        .with_suppressions(policy);
    let engine = match exclusion {
        Some(exclusion) => engine.with_exclusion(exclusion),
        None => engine,
    };
    Ok(if options.timings {
        engine.with_timings()
    } else {
        engine
    })
}

/// Identifies everything that can change a file's diagnostics, for the result cache.
pub fn fingerprint(
    with_astgrep: bool,
    resolution: &Resolution,
    extra_rules: Option<&Path>,
    config: &str,
) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    // The shape of the stored diagnostics: v2 added where each one ends.
    hasher.update(b"diagnostic-v2");
    hasher.update(&[u8::from(with_astgrep)]);
    hasher.update(config.as_bytes());
    for source in catalog_sources(resolution) {
        hasher.update(source.yaml.as_bytes());
        hasher.update(source.prefilter.unwrap_or_default().as_bytes());
    }
    if with_astgrep {
        for yaml in extra_rule_texts(extra_rules)? {
            hasher.update(yaml.as_bytes());
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// The `*.yml` files of the extra rules directory, in name order.
fn extra_rule_texts(dir: Option<&Path>) -> Result<Vec<String>> {
    let Some(dir) = dir else {
        return Ok(Vec::new());
    };
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "yml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| Ok(std::fs::read_to_string(path)?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use botica_catalog::{Catalog, SourceFile};
    use botica_config::Config;

    fn file(path: &str, text: &str) -> SourceFile {
        SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        }
    }

    /// A catalog with one ast-grep rule, `log-no-print`, whose YAML and prefilter are given.
    fn resolution(yaml: &str, prefilter: &str) -> Resolution {
        let rule = format!(
            "id = \"log-no-print\"\ncode = \"BTLOG001\"\nsummary = \"s\"\nstatus = \"stable\"\n\
             {prefilter}scope = \"file\"\nneeds = \"syntax\"\n\
             run-on = [\"cli\"]\ndefault-enabled = true\ndefault-severity = \"error\"\n\n\
             [[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\n\
             file = \"python.yml\"\n"
        );
        let catalog = Catalog::from_files(&[
            file(
                "logging/family.toml",
                "code = \"LOG\"\ndescription = \"d\"\n",
            ),
            file("logging/log-no-print/rule.toml", &rule),
            file("logging/log-no-print/README.md", "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"),
            file("logging/log-no-print/python.yml", yaml),
        ])
        .expect("the catalog loads");
        let config = Config::from_botica_toml("").expect("an empty configuration");
        Resolution::new(&config, &catalog).expect("the catalog resolves")
    }

    fn print_of(resolution: &Resolution) -> String {
        fingerprint(true, resolution, None, "config").expect("the fingerprint")
    }

    const YAML: &str = "id: log-no-print\nlanguage: python\n";

    #[test]
    fn the_fingerprint_changes_with_the_yaml_of_an_active_rule() {
        let same = print_of(&resolution(YAML, ""));
        assert_eq!(same, print_of(&resolution(YAML, "")));

        let edited = format!("{YAML}severity: error\n");
        assert_ne!(same, print_of(&resolution(&edited, "")));
    }

    #[test]
    fn the_fingerprint_changes_with_the_prefilter_of_an_active_rule() {
        let without = print_of(&resolution(YAML, ""));

        assert_ne!(
            without,
            print_of(&resolution(YAML, "prefilter = 'print'\n"))
        );
    }
}
