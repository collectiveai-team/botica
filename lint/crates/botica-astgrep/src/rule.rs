use crate::language::check_languages;
use crate::locality::{inside_reads_contents, is_local, uses_key, uses_key_outside_not};
use crate::prefilter;
use anyhow::{Context, Result};
use ast_grep_config::{from_yaml_string, GlobalRules, RuleConfig, Severity};
use ast_grep_language::SupportLang;
use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub rule_id: String,
    pub severity: &'static str,
    pub start: usize,
    pub end: usize,
    pub message: String,
}

pub struct AstGrepRule {
    pub(crate) config: RuleConfig<SupportLang>,
    pub(crate) files: Option<GlobSet>,
    pub(crate) ignores: Option<GlobSet>,
    pub(crate) requirement: prefilter::Req,
    /// A hand-written requirement (`prefilter` in `rule.toml`); wins over the derived one.
    pub(crate) declared: Option<prefilter::Req>,
    pub(crate) rule_json: serde_json::Value,
    /// Every match lies inside one top-level statement and depends on nothing outside it.
    pub(crate) local: bool,
    /// Reads the contents of a container, so the rule cannot be narrowed below top-level
    /// statements: it uses `has` (the match may be a class whose whole body matters), or an
    /// `inside` that reads its ancestor's text or descendants (a partly parsed class would not
    /// match it, and a `not` around it would then report a false hit).
    pub(crate) inspects_descendants: bool,
    /// Uses `inside` outside a `not`: part of its prefilter lies in an ancestor of the match.
    pub(crate) looks_at_ancestors: bool,
}

impl AstGrepRule {
    pub fn from_yaml(yaml: &str) -> Result<Vec<Self>> {
        check_languages(yaml)?;
        let globals = GlobalRules::default();
        from_yaml_string::<SupportLang>(yaml, &globals)
            .context("invalid ast-grep rule")?
            .into_iter()
            .map(|config| {
                let serialized = serde_json::to_value(&*config)?;
                let files = build_globs(&serialized, "files")?;
                let ignores = build_globs(&serialized, "ignores")?;
                let local = serialized.get("utils").is_none_or(|u| {
                    u.is_null() || u.as_object().is_some_and(serde_json::Map::is_empty)
                }) && serialized.get("rule").is_some_and(is_local);
                let inspects_descendants = serialized.get("rule").is_some_and(|rule| {
                    uses_key(rule, "has") || inside_reads_contents(rule, config.language)
                });
                let looks_at_ancestors = serialized
                    .get("rule")
                    .is_some_and(|rule| uses_key_outside_not(rule, "inside"));
                Ok(Self {
                    looks_at_ancestors,
                    inspects_descendants,
                    local,
                    config,
                    files,
                    ignores,
                    requirement: prefilter::Req::Always,
                    declared: None,
                    rule_json: serialized.get("rule").cloned().unwrap_or_default(),
                })
            })
            .collect()
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    /// Declares the regex the file text must match (the `prefilter` field of `rule.toml`), in
    /// place of the one derived from the rule. Only rules without a pattern need it (`kind:`
    /// only). The caller vouches that every match of the rule implies a match of the regex.
    pub fn declare_prefilter(&mut self, pattern: &str) {
        self.declared = Some(prefilter::Req::Re(pattern.to_owned()));
    }

    pub fn language(&self) -> SupportLang {
        self.config.language
    }

    pub(crate) fn message<D: ast_grep_core::Doc>(
        &self,
        found: &ast_grep_core::NodeMatch<'_, D>,
    ) -> String {
        self.config.get_message(found)
    }

    pub fn applies_to(&self, path: &Path) -> bool {
        let included = self.files.as_ref().is_none_or(|g| g.is_match(path));
        let excluded = self.ignores.as_ref().is_some_and(|g| g.is_match(path));
        included && !excluded
    }
}

/// The `files` and `ignores` globs that the rules of an ast-grep YAML document declare, in the
/// order they are written: the paths each rule looks at, and the paths it leaves out.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PathGlobs {
    pub files: Vec<String>,
    pub ignores: Vec<String>,
}

pub fn path_globs(yaml: &str) -> Result<PathGlobs> {
    let mut globs = PathGlobs::default();
    for config in from_yaml_string::<SupportLang>(yaml, &GlobalRules::default())
        .context("invalid ast-grep rule")?
    {
        let serialized = serde_json::to_value(&*config)?;
        globs.files.extend(glob_texts(&serialized, "files"));
        globs.ignores.extend(glob_texts(&serialized, "ignores"));
    }
    Ok(globs)
}

fn glob_texts<'a>(config: &'a serde_json::Value, key: &str) -> impl Iterator<Item = String> + 'a {
    config
        .get(key)
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| match entry {
            serde_json::Value::String(pattern) => Some(pattern.clone()),
            serde_json::Value::Object(map) => map.get("glob")?.as_str().map(str::to_owned),
            _ => None,
        })
}

/// `RuleFileGlob` is private in ast-grep-config 0.41, so `files`/`ignores` are read back
/// through the config's `Serialize` impl.
fn build_globs(config: &serde_json::Value, key: &str) -> Result<Option<GlobSet>> {
    let Some(entries) = config.get(key).and_then(serde_json::Value::as_array) else {
        return Ok(None);
    };
    let mut builder = GlobSetBuilder::new();
    for entry in entries {
        match entry {
            serde_json::Value::String(pattern) => {
                builder.add(Glob::new(pattern)?);
            }
            serde_json::Value::Object(map) => {
                let pattern = map.get("glob").and_then(|g| g.as_str()).unwrap_or_default();
                let insensitive = map
                    .get("caseInsensitive")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                builder.add(
                    GlobBuilder::new(pattern)
                        .case_insensitive(insensitive)
                        .build()?,
                );
            }
            _ => {}
        }
    }
    Ok(Some(builder.build()?))
}

pub(crate) fn severity_name(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint | Severity::Off => "hint",
    }
}
