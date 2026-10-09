use botica_catalog::{is_kebab_case, Catalog, Rule, Status};

const HEADER: &str = "BT";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    Default,
    All,
    Tag(String),
    Family(String),
    CodePrefix { family: String, digits: String },
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
        if let Some(rest) = text.strip_prefix(HEADER) {
            if let Some(selector) = Self::parse_code(text, rest)? {
                return Ok(selector);
            }
        }
        if is_kebab_case(text) {
            return Ok(Self::Slug(text.to_owned()));
        }
        Err(format!(
            "`{text}` is not a selector: use `default`, `all`, a family (`BTSTRUC`), a code or code prefix (`BTSTRUC001`, `BTSTRUC0`), a rule slug or `tag:<tag>`"
        ))
    }

    fn parse_code(text: &str, rest: &str) -> Result<Option<Self>, String> {
        let letters: String = rest.chars().take_while(char::is_ascii_uppercase).collect();
        let digits = &rest[letters.len()..];
        if !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(None);
        }
        if letters.is_empty() && digits.is_empty() {
            return Err(format!(
                "`{text}` alone selects nothing in particular: use `all` to select every rule"
            ));
        }
        match digits.len() {
            0 => Ok(Some(Self::Family(letters))),
            1 | 2 => Ok(Some(Self::CodePrefix {
                family: letters,
                digits: digits.to_owned(),
            })),
            3 => Ok(Some(Self::Code(text.to_owned()))),
            _ => Err(format!(
                "`{text}` has more than 3 digits: a code is `{HEADER}<FAMILY>` plus 3 digits"
            )),
        }
    }

    pub fn matches(&self, catalog: &Catalog, rule: &Rule) -> bool {
        match self {
            Self::Default => rule.default_enabled,
            Self::All => true,
            Self::Tag(tag) => rule.tags.contains(tag),
            Self::Family(code) => family_of(&rule.code).is_some_and(|family| family == code),
            Self::CodePrefix { family, digits } => {
                rule.code.starts_with(&format!("{HEADER}{family}{digits}"))
            }
            Self::Code(key) | Self::Slug(key) => catalog
                .resolve(key)
                .is_some_and(|found| found.id == rule.id),
        }
    }

    pub fn check(&self, catalog: &Catalog) -> Result<(), String> {
        match self {
            Self::Default | Self::All => Ok(()),
            Self::Tag(tag) => catalog
                .rules()
                .iter()
                .any(|rule| rule.tags.contains(tag))
                .then_some(())
                .ok_or_else(|| format!("no rule has the tag `{tag}`")),
            Self::Family(code) => catalog
                .families()
                .iter()
                .any(|family| &family.code == code)
                .then_some(())
                .ok_or_else(|| format!("unknown family `{HEADER}{code}`")),
            Self::CodePrefix { family, digits } => check_prefix(catalog, family, digits),
            Self::Code(code) => check_rule(catalog.resolve(code), code),
            Self::Slug(slug) => check_rule(catalog.resolve(slug), slug),
        }
    }
}

fn family_of(code: &str) -> Option<&str> {
    let rest = code.strip_prefix(HEADER)?;
    let letters = &rest[..rest
        .find(|c: char| !c.is_ascii_uppercase())
        .unwrap_or(rest.len())];
    (!letters.is_empty()).then_some(letters)
}

fn check_prefix(catalog: &Catalog, family: &str, digits: &str) -> Result<(), String> {
    if !family.is_empty() && !catalog.families().iter().any(|f| f.code == family) {
        return Err(format!("unknown family `{HEADER}{family}`"));
    }
    let prefix = format!("{HEADER}{family}{digits}");
    catalog
        .rules()
        .iter()
        .any(|rule| rule.code.starts_with(&prefix))
        .then_some(())
        .ok_or_else(|| format!("no rule has a code starting with `{prefix}`"))
}

pub(crate) fn check_rule(rule: Option<&Rule>, id: &str) -> Result<(), String> {
    match rule {
        None => Err(format!("unknown rule `{id}`")),
        Some(rule) if rule.status == Status::Removed => {
            Err(format!("rule `{id}` was removed and cannot be mentioned"))
        }
        Some(_) => Ok(()),
    }
}
