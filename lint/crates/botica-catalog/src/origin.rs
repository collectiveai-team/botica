use crate::model::{Implementation, Kind, Rule};

/// What the codes of the built-in catalog start with.
pub(crate) const HEADER: &str = "BT";

/// Where the files of a load come from: the built-in catalog or one catalog of a project.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Origin<'a> {
    BuiltIn,
    Project { name: &'a str, code: &'a str },
}

impl<'a> Origin<'a> {
    /// What the code of every rule starts with, before the code of its family.
    pub(crate) fn header(self) -> &'a str {
        match self {
            Self::BuiltIn => HEADER,
            Self::Project { code, .. } => code,
        }
    }

    pub(crate) fn catalog(self) -> Option<String> {
        match self {
            Self::BuiltIn => None,
            Self::Project { name, .. } => Some(name.to_owned()),
        }
    }

    /// The id of the rule in the folder `slug`: the slug, prefixed by the name of the catalog for a
    /// project.
    pub(crate) fn id_of(self, slug: &str) -> String {
        match self {
            Self::BuiltIn => slug.to_owned(),
            Self::Project { name, .. } => format!("{name}/{slug}"),
        }
    }

    pub(crate) fn is_built_in(self) -> bool {
        matches!(self, Self::BuiltIn)
    }
}

/// What only the built-in catalog or only a catalog of a project may have: the code of a native
/// rule is in botica-core, which a project cannot add to, and an extension is a program that the
/// project runs, which a binary that is installed alone does not have.
pub(crate) fn check_origin(
    index: usize,
    rule: &Rule,
    implementation: &Implementation,
    origin: Origin<'_>,
    add: &mut impl FnMut(String),
) {
    if !origin.is_built_in() && implementation.kind == Kind::Native {
        add(format!(
            "implementation {index}: `{}` is native, and the code of a native rule is in botica-core: a catalog of a project takes the other kinds",
            rule.id
        ));
    }
    if origin.is_built_in() && implementation.kind == Kind::Extension {
        add(format!(
            "implementation {index}: `{}` is an extension, which runs code of the project: a rule of the built-in catalog is native, ast-grep, judgment or external",
            rule.id
        ));
    }
}
