//! Where each tool that botica knows reads its settings, in the order the tool prefers the files,
//! and which of its keys are not compared as plain values. A tool that is not here is read from the
//! one file that the fragment of the rule names.

use super::duration::Unit;

/// A file that a tool reads its settings from, and the table of the file that holds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place<'a> {
    pub file: &'a str,
    pub table: &'a [&'a str],
    /// The tool reads the file whenever it exists, even without the table: `ruff.toml`,
    /// `pytest.ini`. A `pyproject.toml` without the table of the tool is passed over.
    pub always: bool,
    /// The values are text, as in an INI file or in `[tool.pytest.ini_options]`.
    pub text: bool,
}

/// How the settings of a tool hold the fragment of a rule, beyond equal values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    Plain,
    /// `select` and `extend-select` of the fragment are codes of rules, which `ignore` and
    /// `extend-ignore` of the project also account for.
    Ruff,
    /// `preset = "all"` holds `preset = "strict"`, and a kind of error under `errors` holds when
    /// the project sets it, to any severity.
    Pyrefly,
}

/// A key, from the table of the tool, whose value is a time span: the project holds the fragment
/// with a span at least as long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub key: &'static [&'static str],
    /// What a bare number counts; `None` when the tool takes no bare number.
    pub unit: Option<Unit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tool<'a> {
    pub name: &'a str,
    pub places: &'a [Place<'a>],
    /// The key whose value is the path of a file that these settings extend.
    pub extends: Option<&'static str>,
    pub reading: Reading,
    pub spans: &'static [Span],
}

const fn place(file: &'static str, table: &'static [&'static str]) -> Place<'static> {
    Place {
        file,
        table,
        always: table.is_empty(),
        text: false,
    }
}

const fn text(place: Place<'static>) -> Place<'static> {
    Place {
        text: true,
        ..place
    }
}

const fn always(place: Place<'static>) -> Place<'static> {
    Place {
        always: true,
        ..place
    }
}

const fn tool(name: &'static str, places: &'static [Place<'static>]) -> Tool<'static> {
    Tool {
        name,
        places,
        extends: None,
        reading: Reading::Plain,
        spans: &[],
    }
}

/// <https://docs.astral.sh/ruff/configuration/#config-file-discovery>: `.ruff.toml`, then
/// `ruff.toml`, then a `pyproject.toml` with `[tool.ruff]`.
const RUFF: Tool<'static> = Tool {
    extends: Some("extend"),
    reading: Reading::Ruff,
    ..tool(
        "ruff",
        &[
            place(".ruff.toml", &[]),
            place("ruff.toml", &[]),
            place("pyproject.toml", &["tool", "ruff"]),
        ],
    )
};

/// <https://pyrefly.org/en/docs/configuration/#configuration-finding>
const PYREFLY: Tool<'static> = Tool {
    extends: Some("extends"),
    reading: Reading::Pyrefly,
    ..tool(
        "pyrefly",
        &[
            place("pyrefly.toml", &[]),
            place("pyproject.toml", &["tool", "pyrefly"]),
        ],
    )
};

/// <https://docs.pytest.org/en/stable/reference/customize.html#finding-the-rootdir>: the first
/// file that matches wins, and pytest never merges two.
const PYTEST: Tool<'static> = tool(
    "pytest",
    &[
        always(place("pytest.toml", &["pytest"])),
        always(place(".pytest.toml", &["pytest"])),
        text(always(place("pytest.ini", &["pytest"]))),
        text(always(place(".pytest.ini", &["pytest"]))),
        place("pyproject.toml", &["tool", "pytest"]),
        text(place("pyproject.toml", &["tool", "pytest", "ini_options"])),
        text(place("tox.ini", &["pytest"])),
        text(place("setup.cfg", &["tool:pytest"])),
    ],
);

/// <https://docs.astral.sh/uv/concepts/configuration-files/>: `uv.toml` wins over the
/// `[tool.uv]` of `pyproject.toml`.
const UV: Tool<'static> = Tool {
    spans: &[Span {
        key: &["exclude-newer"],
        unit: None,
    }],
    ..tool(
        "uv",
        &[
            place("uv.toml", &[]),
            place("pyproject.toml", &["tool", "uv"]),
        ],
    )
};

/// <https://pnpm.io/settings>: only the settings of auth and registries are read from `.npmrc`.
const PNPM: Tool<'static> = Tool {
    spans: &[Span {
        key: &["minimumReleaseAge"],
        unit: Some(Unit::Minutes),
    }],
    ..tool("pnpm", &[place("pnpm-workspace.yaml", &[])])
};

/// <https://docs.npmjs.com/cli/v11/using-npm/config#min-release-age>
const NPM: Tool<'static> = Tool {
    spans: &[Span {
        key: &["min-release-age"],
        unit: Some(Unit::Days),
    }],
    ..tool("npm", &[text(place(".npmrc", &[]))])
};

/// <https://yarnpkg.com/configuration/yarnrc#npmMinimalAgeGate>: a duration, or a number of minutes.
const YARN: Tool<'static> = Tool {
    spans: &[Span {
        key: &["npmMinimalAgeGate"],
        unit: Some(Unit::Minutes),
    }],
    ..tool("yarn", &[place(".yarnrc.yml", &[])])
};

/// <https://doc.rust-lang.org/cargo/reference/config.html>: "If both files exist, Cargo will use
/// the file without the extension."
const CARGO: Tool<'static> = Tool {
    spans: &[Span {
        key: &["registry", "global-min-publish-age"],
        unit: None,
    }],
    ..tool(
        "cargo",
        &[
            place(".cargo/config", &[]),
            place(".cargo/config.toml", &[]),
        ],
    )
};

/// <https://bun.com/docs/runtime/bunfig>
const BUN: Tool<'static> = Tool {
    spans: &[Span {
        key: &["install", "minimumReleaseAge"],
        unit: Some(Unit::Seconds),
    }],
    ..tool("bun", &[place("bunfig.toml", &[])])
};

const TOOLS: [Tool<'static>; 9] = [RUFF, PYREFLY, PYTEST, UV, PNPM, NPM, YARN, CARGO, BUN];

impl<'a> Tool<'a> {
    /// How botica reads `name` for a fragment that goes into `target`: the table of the tool when
    /// `target` is one of its places, or else `target` alone, compared as plain values.
    pub fn for_target(name: &'a str, target: &'a str, alone: &'a [Place<'a>; 1]) -> Self {
        TOOLS
            .iter()
            .find(|tool| tool.name == name && tool.places.iter().any(|p| p.file == target))
            .copied()
            .unwrap_or(Tool {
                name,
                places: alone,
                extends: None,
                reading: Reading::Plain,
                spans: &[],
            })
    }

    /// The place of `target` with the longest table that the fragment has, or else the first place
    /// of `target`: a fragment under `[tool.pytest.ini_options]` goes there and not under
    /// `[tool.pytest]`.
    pub fn place_of(&self, target: &str, fragment: &toml::Table) -> Place<'a> {
        let of_target = self.places.iter().filter(|place| place.file == target);
        of_target
            .clone()
            .filter(|place| super::lookup(fragment, place.table).is_some())
            .max_by_key(|place| place.table.len())
            .or_else(|| of_target.clone().next())
            .copied()
            .expect("`for_target` keeps a tool that has a place for the target")
    }

    /// The keys of the table of `place` that are the tables of other places of the same file, as
    /// `ini_options` is for `[tool.pytest]`.
    pub fn nested(&self, place: &Place<'_>) -> Vec<&'a str> {
        self.places
            .iter()
            .filter(|other| other.file == place.file && other.table.len() == place.table.len() + 1)
            .filter(|other| other.table.starts_with(place.table))
            .map(|other| other.table[place.table.len()])
            .collect()
    }
}

/// The one place of a file that no tool of the table reads: the whole file.
pub fn alone(target: &str) -> [Place<'_>; 1] {
    [Place {
        file: target,
        table: &[],
        always: true,
        text: matches!(
            botica_catalog::Format::of(target),
            Some(botica_catalog::Format::Ini | botica_catalog::Format::Npmrc)
        ),
    }]
}
