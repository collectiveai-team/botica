use anyhow::Result;
use botica_astgrep::{AstGrepRule, AstGrepSet};
use botica_core::{Engine, FileSizeGuard, NativeRule, NoUtils};

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

pub fn build_engine(with_astgrep: bool, prefilter: bool) -> Result<Engine> {
    let native: Vec<Box<dyn NativeRule>> =
        vec![Box::new(FileSizeGuard::default()), Box::new(NoUtils)];
    let mut rules = Vec::new();
    if with_astgrep {
        for yaml in EMBEDDED {
            for mut rule in AstGrepRule::from_yaml(yaml)? {
                if let Some(literals) = declared_literals(rule.id()) {
                    rule.require(literals);
                }
                rules.push(rule);
            }
        }
    }
    let mut set = AstGrepSet::new(rules);
    set.set_prefilter(prefilter);
    Ok(Engine::new(native, set))
}

/// Identifies everything that can change a file's diagnostics, for the result cache.
pub fn fingerprint(with_astgrep: bool) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(&[u8::from(with_astgrep)]);
    hasher.update(b"file-size-guard:400:700");
    for yaml in EMBEDDED {
        hasher.update(yaml.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

/// What a `prefilter = [...]` field in `rule.toml` would carry: rules written with `kind:` only
/// have no pattern for the filter to read, so their required literals are stated by hand.
fn declared_literals(rule_id: &str) -> Option<&'static [&'static str]> {
    match rule_id {
        "no-dict-literal-return" => Some(&["return", "{"]),
        _ => None,
    }
}
