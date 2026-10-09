use anyhow::Result;
use botica_astgrep::{AstGrepRule, AstGrepSet};
use botica_core::suppress::Policy;
use botica_core::{Engine, Exclusion, NativePlanner, NativeRule};
use std::path::Path;

const EMBEDDED: [&str; 11] = [
    include_str!("../../../../scaffolding/templates/ast-grep-rules/api-schemas-extra-forbid.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/arch-database-package.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/cli-typed-framework.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/log-get-logger.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/log-no-print.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-alias.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-call-return.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-literal-return.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-return-annotation.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/settings-module.yml"),
    include_str!("../rules/no-console-log.yml"),
];

pub struct EngineOptions<'a> {
    pub astgrep: bool,
    pub prefilter: bool,
    /// Extra ast-grep YAML rules (`*.yml` in this directory), for parity checks on more languages.
    pub extra_rules: Option<&'a Path>,
    /// Print each rule's effective prefilter to stderr.
    pub describe_prefilter: bool,
    /// Parse only the top-level statements holding a candidate (Python, JS/TS).
    pub regions: bool,
    /// Skip, while matching, the subtrees without a candidate position.
    pub subtree_skip: bool,
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
        for yaml in EMBEDDED {
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
    let policy = policy.with_astgrep_rules(rules.iter().map(AstGrepRule::id));
    let mut set = AstGrepSet::new(rules);
    if options.describe_prefilter {
        for (id, requirement) in set.describe_prefilter() {
            eprintln!("[prefilter] {id}: {requirement}");
        }
    }
    set.set_prefilter(options.prefilter);
    set.set_regions(options.regions && options.prefilter);
    set.set_skip_subtrees(options.subtree_skip && options.prefilter);
    let engine = Engine::new(native, set)
        .with_planner(planner)
        .with_suppressions(policy);
    Ok(match exclusion {
        Some(exclusion) => engine.with_exclusion(exclusion),
        None => engine,
    })
}

/// Identifies everything that can change a file's diagnostics, for the result cache.
pub fn fingerprint(with_astgrep: bool, extra_rules: Option<&Path>, config: &str) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    // The shape of the stored diagnostics: v2 added where each one ends.
    hasher.update(b"diagnostic-v2");
    hasher.update(&[u8::from(with_astgrep)]);
    hasher.update(config.as_bytes());
    for yaml in EMBEDDED {
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
