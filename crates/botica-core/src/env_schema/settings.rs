//! The settings classes of a project, in terms that no language owns: a reader of a language gives
//! the classes it finds in a file, and this module works out which environment variables their
//! fields read.

/// A value of the configuration of a class that the reader could only read when it is a literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Text {
    Known(String),
    Unknown,
}

/// The configuration of a class as written in it; a `None` is a key it leaves to its parents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub env_prefix: Option<Text>,
    /// Set (to anything but nothing): nested values come from variables split by it.
    pub nested_delimiter: Option<Text>,
    pub case_sensitive: Option<bool>,
    /// The names come from something the reader cannot follow (an alias generator, an old-style
    /// configuration class).
    pub opaque: bool,
}

impl Config {
    /// `self` with what `child` sets on top.
    fn under(&self, child: &Self) -> Self {
        Self {
            env_prefix: child.env_prefix.clone().or_else(|| self.env_prefix.clone()),
            nested_delimiter: child
                .nested_delimiter
                .clone()
                .or_else(|| self.nested_delimiter.clone()),
            case_sensitive: child.case_sensitive.or(self.case_sensitive),
            opaque: self.opaque || child.opaque,
        }
    }
}

/// The name a field gives to its variable in place of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alias {
    None,
    Name(String),
    /// One of these names, or a path into one of them.
    Choices(Vec<String>),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    /// The byte range of the name.
    pub start: usize,
    pub end: usize,
    pub alias: Alias,
    /// The names its type mentions, to find a field that holds the settings of another class.
    pub type_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    /// The file that defines it.
    pub path: String,
    pub name: String,
    /// The last part of the name of each base class.
    pub bases: Vec<String>,
    /// It extends the base class of the settings library itself.
    pub extends_root: bool,
    pub config: Config,
    pub fields: Vec<Field>,
}

/// How a variable name may be read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pattern {
    Exact { name: String, case_sensitive: bool },
    Prefix(String),
    Suffix(String),
}

impl Pattern {
    fn matches(&self, variable: &str) -> bool {
        let lower = variable.to_ascii_lowercase();
        match self {
            Self::Exact {
                name,
                case_sensitive: true,
            } => name == variable,
            Self::Exact { name, .. } => name.to_ascii_lowercase() == lower,
            Self::Prefix(prefix) => lower.starts_with(&prefix.to_ascii_lowercase()),
            Self::Suffix(suffix) => lower.ends_with(&suffix.to_ascii_lowercase()),
        }
    }
}

/// A field as declared in one class, with every name that it or a subclass may read.
#[derive(Debug, Clone)]
pub struct Declaration<'a> {
    pub path: &'a str,
    pub class: &'a str,
    pub field: &'a Field,
    /// The variable it reads in the class that declares it, as a person would write it.
    pub expected: String,
    /// Every name it reads is known.
    determined: bool,
    patterns: Vec<Pattern>,
}

impl Declaration<'_> {
    /// Whether it reads `variable`.
    pub fn reads(&self, variable: &str) -> bool {
        self.patterns
            .iter()
            .any(|pattern| pattern.matches(variable))
    }

    /// Whether its names are all known, so a missing variable can be reported.
    pub const fn determined(&self) -> bool {
        self.determined
    }
}

/// What a field reads in a class with `config`.
struct Read {
    patterns: Vec<Pattern>,
    determined: bool,
    expected: String,
}

fn read(config: &Config, field: &Field) -> Read {
    let case_sensitive = config.case_sensitive == Some(true);
    let exact = |name: &str| Pattern::Exact {
        name: name.to_owned(),
        case_sensitive,
    };
    let shown = |name: &str| {
        if case_sensitive {
            name.to_owned()
        } else {
            name.to_ascii_uppercase()
        }
    };
    let mut determined = config.nested_delimiter.is_none();
    let mut patterns = Vec::new();
    let base = match &field.alias {
        Alias::Name(name) => {
            patterns.push(exact(name));
            name.clone()
        }
        Alias::Choices(names) => {
            determined = false;
            patterns.extend(names.iter().map(|name| exact(name)));
            names.first().cloned().unwrap_or_else(|| field.name.clone())
        }
        Alias::Unknown => {
            determined = false;
            field.name.clone()
        }
        Alias::None => match (&config.env_prefix, config.opaque) {
            (Some(Text::Unknown), _) | (_, true) => {
                determined = false;
                patterns.push(Pattern::Suffix(field.name.clone()));
                field.name.clone()
            }
            (Some(Text::Known(prefix)), false) => {
                let name = format!("{prefix}{}", field.name);
                patterns.push(exact(&name));
                name
            }
            (None, false) => {
                patterns.push(exact(&field.name));
                field.name.clone()
            }
        },
    };
    if let Some(delimiter) = &config.nested_delimiter {
        patterns.push(Pattern::Prefix(match delimiter {
            Text::Known(delimiter) => format!("{base}{delimiter}"),
            Text::Unknown => base.clone(),
        }));
    }
    Read {
        patterns,
        determined,
        expected: shown(&base),
    }
}

/// The classes of `classes` that are settings classes: those that extend the root class, and
/// those that extend one of them.
fn settings_names(classes: &[Class]) -> Vec<&str> {
    let mut names: Vec<&str> = classes
        .iter()
        .filter(|class| class.extends_root)
        .map(|class| class.name.as_str())
        .collect();
    loop {
        let more: Vec<&str> = classes
            .iter()
            .filter(|class| !names.contains(&class.name.as_str()))
            .filter(|class| {
                class
                    .bases
                    .iter()
                    .any(|base| names.contains(&base.as_str()))
            })
            .map(|class| class.name.as_str())
            .collect();
        if more.is_empty() {
            return names;
        }
        names.extend(more);
    }
}

/// `class` and the settings classes it extends, the farthest first.
fn lineage<'a>(classes: &'a [Class], settings: &[&str], class: &'a Class) -> Vec<&'a Class> {
    let mut chain = vec![class];
    let mut index = 0;
    while let Some(current) = chain.get(index) {
        let parents: Vec<&Class> = current
            .bases
            .iter()
            .filter(|base| settings.contains(&base.as_str()))
            .filter_map(|base| classes.iter().find(|known| known.name == *base))
            .filter(|parent| !chain.iter().any(|seen| std::ptr::eq(*seen, *parent)))
            .collect();
        chain.extend(parents);
        index += 1;
    }
    chain.reverse();
    chain
}

/// The fields of the settings classes, one for each place a field is declared, and whether the
/// project has a settings class at all.
pub fn declarations(classes: &[Class]) -> (Vec<Declaration<'_>>, bool) {
    let settings = settings_names(classes);
    let mut found: Vec<Declaration<'_>> = Vec::new();
    for class in classes
        .iter()
        .filter(|c| settings.contains(&c.name.as_str()))
    {
        let chain = lineage(classes, &settings, class);
        let config = chain
            .iter()
            .fold(Config::default(), |config, link| config.under(&link.config));
        for owner in &chain {
            for field in &owner.fields {
                if field
                    .type_names
                    .iter()
                    .any(|name| settings.contains(&name.as_str()))
                {
                    continue;
                }
                let read = read(&config, field);
                match found
                    .iter_mut()
                    .find(|known| std::ptr::eq(known.field, field))
                {
                    Some(known) => {
                        known.determined &= read.determined;
                        known.patterns.extend(read.patterns);
                        if std::ptr::eq(*owner, class) {
                            known.expected = read.expected;
                        }
                    }
                    None => found.push(Declaration {
                        path: &owner.path,
                        class: &owner.name,
                        field,
                        expected: read.expected,
                        determined: read.determined,
                        patterns: read.patterns,
                    }),
                }
            }
        }
    }
    (found, !settings.is_empty())
}
