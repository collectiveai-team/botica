use botica_catalog::{is_kebab_case, Catalog, Rule, Status};

/// What a selector names. The letters of a code carry the code of its catalog: `BTSTRUC` is the
/// family `STRUC` of the built-in catalog and `ACMESTRUC` the one of the catalog `ACME`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    Default,
    All,
    Tag(String),
    Family(String),
    CodePrefix { letters: String, digits: String },
    Code(String),
    Slug(String),
}

impl Selector {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "default" => return Ok(Self::Default),
            "all" => return Ok(Self::All),
            _ => {}
        }
        if let Some(tag) = text.strip_prefix("tag:") {
            return if is_kebab_case(tag) {
                Ok(Self::Tag(tag.to_owned()))
            } else {
                Err(format!(
                    "`{text}`: a tag is a kebab-case word, as in `tag:size`"
                ))
            };
        }
        if let Some(selector) = Self::parse_code(text)? {
            return Ok(selector);
        }
        if is_rule_id(text) {
            return Ok(Self::Slug(text.to_owned()));
        }
        Err(format!(
            "`{text}` is not a selector: use `default`, `all`, a family (`BTSTRUC`), a code or code prefix (`BTSTRUC001`, `BTSTRUC0`), a rule id (`no-utils`, or `acme/no-utils` in a catalog of the project) or `tag:<tag>`"
        ))
    }

    /// A code, a code prefix or a family: capital letters, then up to 3 digits.
    fn parse_code(text: &str) -> Result<Option<Self>, String> {
        let letters: String = text.chars().take_while(char::is_ascii_uppercase).collect();
        let digits = &text[letters.len()..];
        if letters.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(None);
        }
        match digits.len() {
            0 => Ok(Some(Self::Family(letters))),
            1 | 2 => Ok(Some(Self::CodePrefix {
                letters,
                digits: digits.to_owned(),
            })),
            3 => Ok(Some(Self::Code(text.to_owned()))),
            _ => Err(format!(
                "`{text}` has more than 3 digits: a code is the letters of its family plus 3 digits"
            )),
        }
    }

    pub fn matches(&self, catalog: &Catalog, rule: &Rule) -> bool {
        match self {
            Self::Default => rule.default_enabled,
            Self::All => true,
            Self::Tag(tag) => rule.tags.contains(tag),
            Self::Family(prefix) => letters_of(&rule.code) == prefix,
            Self::CodePrefix { letters, digits } => {
                rule.code.starts_with(&format!("{letters}{digits}"))
            }
            Self::Code(key) | Self::Slug(key) => catalog
                .resolve(key)
                .is_some_and(|found| found.id == rule.id),
        }
    }

    /// Checks what the selector names: an error, or the warning that reading it prints. `use`
    /// says whether the selector turns rules on or leaves them off.
    pub fn check(&self, catalog: &Catalog, how: Use) -> Result<Option<String>, String> {
        match self {
            Self::Default | Self::All => Ok(None),
            Self::Tag(tag) => catalog
                .rules()
                .iter()
                .any(|rule| rule.tags.contains(tag))
                .then_some(None)
                .ok_or_else(|| format!("no rule has the tag `{tag}`")),
            Self::Family(prefix) => check_family(catalog, prefix).map(|()| None),
            Self::CodePrefix { letters, digits } => {
                check_prefix(catalog, letters, digits).map(|()| None)
            }
            Self::Code(key) | Self::Slug(key) => read_rule(catalog, key, how),
        }
    }
}

/// A slug, or a slug prefixed by the name of a catalog of the project: `no-utils`,
/// `acme/no-utils`.
fn is_rule_id(text: &str) -> bool {
    match text.split_once('/') {
        Some((catalog, slug)) => is_kebab_case(catalog) && is_kebab_case(slug),
        None => is_kebab_case(text),
    }
}

/// The capital letters a code starts with: the code of its catalog and of its family.
fn letters_of(code: &str) -> &str {
    &code[..code
        .find(|c: char| !c.is_ascii_uppercase())
        .unwrap_or(code.len())]
}

fn check_family(catalog: &Catalog, prefix: &str) -> Result<(), String> {
    if catalog.families().iter().any(|f| f.prefix() == prefix) {
        Ok(())
    } else if catalog.has_header(prefix) {
        Err(format!(
            "`{prefix}` alone selects nothing in particular: use `all` to select every rule"
        ))
    } else {
        Err(format!("unknown family `{prefix}`"))
    }
}

fn check_prefix(catalog: &Catalog, letters: &str, digits: &str) -> Result<(), String> {
    let known =
        catalog.has_header(letters) || catalog.families().iter().any(|f| f.prefix() == letters);
    if !known {
        return Err(format!("unknown family `{letters}`"));
    }
    let prefix = format!("{letters}{digits}");
    catalog
        .rules()
        .iter()
        .any(|rule| rule.code.starts_with(&prefix))
        .then_some(())
        .ok_or_else(|| format!("no rule has a code starting with `{prefix}`"))
}

/// How a place of the configuration uses the name of a rule (ADR 0016, decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// It turns the rule on: `select`, `extend-select`, `--select`, `--extend-select`,
    /// `enabled = true`. A removed rule there is an error.
    TurnsOn,
    /// The same, read by a command that describes the configuration (`botica rules`, `botica
    /// explain`, `botica recipe check`): a removed rule there is a warning, so the command runs.
    TurnsOnDescribed,
    /// Anything else: `ignore`, `--ignore`, `[rules.<name>]`, `[[overrides]]`, `replace`. A removed
    /// rule there is a warning and changes nothing.
    Other,
}

/// Reads the name of a rule: an error when no rule has it or when it turns on a removed rule, a
/// warning when it names a removed rule elsewhere or is a retired name, nothing otherwise.
pub(crate) fn read_rule(catalog: &Catalog, name: &str, how: Use) -> Result<Option<String>, String> {
    let Some(rule) = catalog.resolve(name) else {
        return Err(format!("unknown rule `{name}`"));
    };
    if rule.status == Status::Removed {
        let message = removed(catalog, rule);
        return if how == Use::TurnsOn {
            Err(message)
        } else {
            Ok(Some(message))
        };
    }
    Ok(catalog.retired_warning(name))
}

/// `rule `no-console-log` was removed in 0.10.0 and `log-no-print` replaces it`.
fn removed(catalog: &Catalog, rule: &Rule) -> String {
    let when = catalog
        .removed_in(rule)
        .map_or_else(String::new, |version| format!(" in {version}"));
    let by = catalog
        .rules()
        .iter()
        .find(|other| other.replaces.as_deref() == Some(rule.id.as_str()))
        .map_or_else(String::new, |other| {
            format!(" and `{}` replaces it", other.id)
        });
    format!(
        "rule `{}` was removed{when}{by}, so it no longer runs",
        rule.id
    )
}
