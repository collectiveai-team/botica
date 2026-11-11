use schemars::JsonSchema;
use serde::Deserialize;

use crate::matcher::PathMatcher;

/// How much review a change needs (ADR 0018).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Low,
    Medium,
    High,
}

impl Tier {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// The four areas of ADR 0018, each with a floor under which no entry can put its tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AreaName {
    Critical,
    Platform,
    Core,
    Periphery,
}

impl AreaName {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Platform => "platform",
            Self::Core => "core",
            Self::Periphery => "periphery",
        }
    }

    pub const fn floor(self) -> Tier {
        match self {
            Self::Critical => Tier::High,
            Self::Platform | Self::Core => Tier::Medium,
            Self::Periphery => Tier::Low,
        }
    }

    /// The areas that need people who answer for them.
    pub const fn needs_owners(self) -> bool {
        matches!(self, Self::Critical | Self::Platform)
    }
}

/// One entry of `[[review.areas]]`. An area name may appear in several entries.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ReviewArea {
    pub name: AreaName,
    /// Globs of the paths of the area, as in `applies.paths` of a `REVIEW.md`.
    pub paths: Vec<String>,
    /// The people or teams that answer for the paths. Required for `critical` and `platform`.
    #[serde(default)]
    pub owners: Vec<String>,
    /// Default: the floor of the area. A lower tier than the floor is an error.
    pub tier: Option<Tier>,
    /// The approvals that the paths ask for. Default: 1, and 0 for `periphery`.
    pub approvals: Option<u32>,
}

impl ReviewArea {
    pub fn effective_tier(&self) -> Tier {
        self.tier.unwrap_or_else(|| self.name.floor())
    }

    pub fn effective_approvals(&self) -> u32 {
        self.approvals
            .unwrap_or(if self.name == AreaName::Periphery {
                0
            } else {
                1
            })
    }
}

/// The roles of ADR 0017.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReviewRole {
    Spec,
    Security,
    Tests,
    Architecture,
    Contracts,
    Performance,
    Refute,
}

impl ReviewRole {
    pub const ALL: [Self; 7] = [
        Self::Spec,
        Self::Security,
        Self::Tests,
        Self::Architecture,
        Self::Contracts,
        Self::Performance,
        Self::Refute,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Spec => "spec",
            Self::Security => "security",
            Self::Tests => "tests",
            Self::Architecture => "architecture",
            Self::Contracts => "contracts",
            Self::Performance => "performance",
            Self::Refute => "refute",
        }
    }
}

/// What the classifier does with its decision (ADR 0018 decision 13).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ClassifierMode {
    /// Decides and writes the report, and never merges.
    #[default]
    Shadow,
    Enforce,
}

/// Settings of the classifier that runs after the checks (ADR 0018 decisions 6 and 8).
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct ClassifierConfig {
    pub mode: ClassifierMode,
    /// The checks that must be present and green. An empty list sends every pull request to a
    /// person.
    pub required_checks: Vec<String>,
    /// `u`: a question answers `true` with a probability under this one. At most 0.3.
    #[schemars(range(min = 0.0, max = 0.3))]
    pub question_max: f64,
    /// `U`: the probability that a mistake shows first to the team or to whoever develops the
    /// repository, at least. Not under 0.7.
    #[schemars(range(min = 0.7, max = 1.0))]
    pub impact_min: f64,
    /// A `choice` answer with less confidence than this goes to a person. Not under 0.7.
    #[schemars(range(min = 0.7, max = 1.0))]
    pub min_confidence: f64,
    /// Larger changes go to a person. At most 1000.
    #[schemars(range(max = 1000))]
    pub max_changed_lines: u32,
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            mode: ClassifierMode::Shadow,
            required_checks: Vec::new(),
            question_max: 0.1,
            impact_min: 0.9,
            min_confidence: 0.7,
            max_changed_lines: 400,
        }
    }
}

/// The `review` table: areas, roles and classifier (ADR 0017 and ADR 0018).
#[derive(Debug, Clone, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ReviewConfig {
    /// The folder of the review roles, from the root of the project, when it is not
    /// `.github/review` or `.gitlab/review`.
    #[serde(default)]
    pub dir: Option<String>,
    /// The roles that CI expects. Default: the seven roles.
    #[serde(default)]
    pub roles: Option<Vec<ReviewRole>>,
    /// Globs of generated files, which the size of a change does not count. They cannot match a
    /// path of `critical` or `platform`.
    #[serde(default)]
    pub generated: Vec<String>,
    #[serde(default)]
    pub areas: Vec<ReviewArea>,
    #[serde(default)]
    pub classifier: ClassifierConfig,
}

/// Paths that are `platform` whatever the entries say, since they decide how a pull request is
/// reviewed, built and merged, or are loaded by an agent as instructions (ADR 0018 decision 1).
/// A name with no `/` matches at every depth, as in a `.gitignore`, so a `botica.toml` of a
/// subfolder counts too.
const FORCED_PLATFORM: [&str; 21] = [
    "botica.toml",
    "/pyproject.toml",
    ".github/review/**",
    ".gitlab/review/**",
    "skills-lock.json",
    ".github/workflows/**",
    ".github/actions/**",
    ".gitlab-ci.yml",
    ".github/CODEOWNERS",
    ".gitlab/CODEOWNERS",
    ".gitattributes",
    "prek.toml",
    ".abide/**",
    "**/AGENTS.md",
    "**/CLAUDE.md",
    "**/.claude/**",
    "**/.mcp.json",
    "**/.cursorrules",
    "**/INVARIANTS.md",
    "**/REVIEW.md",
    "**/SKILL.md",
];

/// A glob of the table that selects no file of the list it was checked against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmatchedPath {
    /// Where the glob is written, as `review.areas[1].paths[0]` or `review.generated[2]`.
    pub location: String,
    pub glob: String,
}

impl ReviewConfig {
    /// The globs of `generated` and of the paths of the areas that match none of `files`, which
    /// are paths from the root of the project. A glob that matches nothing is a typo or a path
    /// that moved, and the area it belongs to protects nothing.
    pub fn unmatched_paths(&self, files: &[&str]) -> Vec<UnmatchedPath> {
        let globs =
            self.generated
                .iter()
                .enumerate()
                .map(|(index, glob)| (format!("review.generated[{index}]"), glob))
                .chain(self.areas.iter().enumerate().flat_map(|(index, area)| {
                    area.paths.iter().enumerate().map(move |(path, glob)| {
                        (format!("review.areas[{index}].paths[{path}]"), glob)
                    })
                }));
        globs
            .filter(|(_, glob)| {
                PathMatcher::new(&[(*glob).clone()])
                    .is_ok_and(|matcher| !files.iter().any(|file| matcher.is_match(file)))
            })
            .map(|(location, glob)| UnmatchedPath {
                location,
                glob: glob.clone(),
            })
            .collect()
    }

    /// The roles that CI expects.
    pub fn expected_roles(&self) -> Vec<ReviewRole> {
        self.roles
            .clone()
            .unwrap_or_else(|| ReviewRole::ALL.to_vec())
    }

    /// The globs of the paths that are `platform` whatever the entries say, with the folder of
    /// `dir` when the table sets one.
    pub fn forced_platform_globs(&self) -> Vec<String> {
        let mut globs: Vec<String> = FORCED_PLATFORM
            .iter()
            .map(|glob| (*glob).to_owned())
            .collect();
        if let Some(dir) = &self.dir {
            globs.push(format!("{dir}/**"));
        }
        globs
    }
}
