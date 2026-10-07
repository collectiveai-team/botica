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
                if std::env::var_os("BOTICA_PREFILTER_AUTO").is_some() {
                    if rule.id() == "no-dict-literal-return" {
                        rule.require_regex(declared_regex(rule.id()).expect("declared"));
                    }
                } else if std::env::var_os("BOTICA_PREFILTER_REGEX").is_some() {
                    if let Some(pattern) = declared_regex(rule.id()) {
                        rule.require_regex(pattern);
                    }
                } else if let Some(literals) = declared_literals(rule.id()) {
                    rule.require(literals);
                }
                rules.push(rule);
            }
        }
    }
    if let Some(dir) = std::env::var_os("BOTICA_EXTRA_RULES").filter(|_| with_astgrep) {
        // Test hook: extra ast-grep YAML rules, e.g. to check prefilter parity on more languages.
        let mut paths: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "yml"))
            .collect();
        paths.sort();
        for path in paths {
            rules.extend(AstGrepRule::from_yaml(&std::fs::read_to_string(path)?)?);
        }
    }
    let mut set = AstGrepSet::new(rules);
    if std::env::var_os("BOTICA_PREFILTER_AUTO").is_some() {
        set.use_auto_prefilter();
        if std::env::var_os("BOTICA_PREFILTER_DESCRIBE").is_some() {
            for (id, requirement) in set.describe_prefilter() {
                eprintln!("[prefilter] {id}: {requirement}");
            }
        }
    }
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

/// Prototype of a `prefilter = "<regex>"` field: a necessary condition on the file text for the
/// rule to match anywhere (ASCII byte regex). Experimental, hand-checked against the rules.
fn declared_regex(rule_id: &str) -> Option<&'static str> {
    Some(match rule_id {
        "api-schemas-extra-forbid" => "BaseModel",
        "arch-database-package" => "SQLModel|create_engine|sessionmaker",
        "cli-typed-framework" => "argparse|click|argv",
        "log-get-logger" => "getLogger",
        "log-no-print" => r"(?-u)\bprint\s*\(",
        "no-dict-alias" => r"(?-u)=\s*dict\b",
        "no-dict-call-return" => r"(?-u)\breturn[\s\\]+dict",
        "no-dict-literal-return" => r"(?-u)\breturn[\s\\]*\{",
        "no-dict-return-annotation" => r"(?-u)->\s*dict\b",
        "settings-module" => r"(?-u)\bos\s*\.\s*(getenv|environ)",
        _ => return None,
    })
}

/// What a `prefilter = [...]` field in `rule.toml` would carry: rules written with `kind:` only
/// have no pattern for the filter to read, so their required literals are stated by hand.
fn declared_literals(rule_id: &str) -> Option<&'static [&'static str]> {
    match rule_id {
        "no-dict-literal-return" => Some(&["return", "{"]),
        _ => None,
    }
}
