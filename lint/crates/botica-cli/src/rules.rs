use anyhow::Result;
use botica_astgrep::{AstGrepRule, AstGrepSet, Tuning};
use botica_catalog::Kind;
use botica_config::Resolution;
use botica_core::suppress::Policy;
use botica_core::{Engine, Exclusion, NativePlanner, NativeRule};
use std::path::Path;

/// The syntax rules that are not in the catalog yet.
const EMBEDDED: [&str; 10] = [
    include_str!("../../../../scaffolding/templates/ast-grep-rules/api-schemas-extra-forbid.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/arch-database-package.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/cli-typed-framework.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/log-get-logger.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-alias.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-call-return.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-literal-return.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-return-annotation.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/settings-module.yml"),
    include_str!("../rules/no-console-log.yml"),
];

/// The IDs of the embedded rules, known to the suppression comments whether or not the rules run.
/// Read from each file's `id:` line so that a run without ast-grep need not compile them.
fn embedded_slugs() -> impl Iterator<Item = &'static str> {
    EMBEDDED
        .iter()
        .flat_map(|yaml| yaml.lines().filter_map(|line| line.strip_prefix("id: ")))
        .map(str::trim)
}

/// The slug and the YAML of every active rule of the catalog that is implemented with ast-grep.
pub fn catalog_sources(resolution: &Resolution) -> Vec<(&str, &str)> {
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
                .map(|yaml| (entry.rule.id.as_str(), yaml))
        })
        .collect()
}

pub struct EngineOptions<'a> {
    pub astgrep: bool,
    /// Whether the selection takes in the embedded rules that are not in the catalog yet: no
    /// selector can name them, so they run only with `default` or `all`.
    pub embedded: bool,
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
    let from_catalog = catalog_sources(options.resolution);
    let mut rules = Vec::new();
    if options.astgrep {
        let embedded = if options.embedded { &EMBEDDED[..] } else { &[] };
        let catalog_yaml = from_catalog.iter().map(|&(_, yaml)| yaml);
        for yaml in catalog_yaml.chain(embedded.iter().copied()) {
            for mut rule in AstGrepRule::from_yaml(yaml)? {
                if let Some(pattern) = declared_prefilter(rule.id()) {
                    rule.declare_prefilter(pattern);
                }
                rules.push(rule);
            }
        }
        for yaml in extra_rule_texts(options.extra_rules)? {
            rules.extend(AstGrepRule::from_yaml(&yaml)?);
        }
    }
    let policy = policy
        .with_astgrep_rules(embedded_slugs())
        .with_astgrep_rules(rules.iter().map(AstGrepRule::id));
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
    for yaml in EMBEDDED {
        hasher.update(yaml.as_bytes());
    }
    for (_, yaml) in catalog_sources(resolution) {
        hasher.update(yaml.as_bytes());
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

/// Stand-in for the `prefilter = "<regex>"` field of `rule.toml`, which only rules without a
/// pattern need: the derived prefilter has nothing to read in a `kind:`-only rule.
fn declared_prefilter(rule_id: &str) -> Option<&'static str> {
    match rule_id {
        "no-dict-literal-return" => Some(r"(?-u)\breturn[\s\\]*\{"),
        _ => None,
    }
}
