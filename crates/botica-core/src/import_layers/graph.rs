//! The modules of the top packages of the layers and what each one imports: the module of each
//! Python file, found from the folder that holds its top package, and its imports resolved to the
//! modules of the project.

use std::collections::{BTreeMap, HashMap, VecDeque};

use super::layers::{within, Breach, Layers, Place};
use super::python::{imports, Import};
use super::Source;

/// A module of the project: its dotted name, its file and its imports, each with the modules it
/// names.
pub struct Module<'s> {
    pub name: String,
    pub source: &'s Source<'s>,
    pub imports: Vec<(Import, Vec<String>)>,
}

/// The folder that holds the top package `top`, of all those where the files put it: the root of
/// the project, then `src`, then the shallowest.
fn root_of<'p>(top: &str, paths: &[Vec<&'p str>]) -> Option<Vec<&'p str>> {
    let mut found: Vec<Vec<&str>> = paths
        .iter()
        .filter_map(|parts| {
            let at = parts.iter().position(|part| is_top(part, top))?;
            Some(parts[..at].to_vec())
        })
        .collect();
    found.sort_by_key(|prefix| {
        (
            !prefix.is_empty(),
            *prefix != ["src"],
            prefix.len(),
            prefix.join("/"),
        )
    });
    found.into_iter().next()
}

fn is_top(part: &str, top: &str) -> bool {
    part == top
        || part
            .strip_prefix(top)
            .is_some_and(|rest| rest == ".py" || rest == ".pyi")
}

/// The dotted name of the file whose path from the folder of its top package is `parts`, and
/// whether it is a package (`__init__`).
fn name_of(parts: &[&str]) -> (String, bool) {
    let mut names: Vec<&str> = parts.to_vec();
    if let Some(last) = names.last_mut() {
        *last = last
            .strip_suffix(".pyi")
            .or_else(|| last.strip_suffix(".py"))
            .unwrap_or(last);
    }
    let package = names.last() == Some(&"__init__");
    if package {
        names.pop();
    }
    (names.join("."), package)
}

/// The package that a relative import of `level` starts from, in the module `name`.
fn base_of(name: &str, package: bool, level: usize) -> Option<String> {
    let mut base = if package {
        name.to_owned()
    } else {
        name.rsplit_once('.')?.0.to_owned()
    };
    for _ in 1..level {
        base = base.rsplit_once('.')?.0.to_owned();
    }
    Some(base)
}

pub struct Graph<'s> {
    pub modules: Vec<Module<'s>>,
    index: HashMap<String, usize>,
}

impl<'s> Graph<'s> {
    /// The modules of the files of `sources` that are inside a top package of `layers`, in the
    /// order of their names.
    pub fn build(layers: &Layers, sources: &'s [Source<'s>]) -> Self {
        let tops = layers.top_packages();
        let paths: Vec<Vec<&str>> = sources
            .iter()
            .map(|source| source.path.split('/').collect())
            .collect();
        let roots: Vec<(&str, Vec<&str>)> = tops
            .iter()
            .filter_map(|&top| Some((top, root_of(top, &paths)?)))
            .collect();
        let mut found: BTreeMap<String, (&Source<'_>, bool)> = BTreeMap::new();
        for (source, parts) in sources.iter().zip(&paths) {
            let inside = roots.iter().find(|(top, root)| {
                parts.len() > root.len()
                    && parts[..root.len()] == root[..]
                    && is_top(parts[root.len()], top)
            });
            if let Some((_, root)) = inside {
                let (name, package) = name_of(&parts[root.len()..]);
                found.insert(name, (source, package));
            }
        }
        let known: Vec<&str> = found.keys().map(String::as_str).collect();
        let modules: Vec<Module<'_>> = found
            .iter()
            .map(|(name, &(source, package))| Module {
                name: name.clone(),
                source,
                imports: imports(source.text)
                    .into_iter()
                    .map(|import| {
                        let targets = targets(&import, name, package, &known, &tops);
                        (import, targets)
                    })
                    .collect(),
            })
            .collect();
        let index = modules
            .iter()
            .enumerate()
            .map(|(at, module)| (module.name.clone(), at))
            .collect();
        Self { modules, index }
    }

    /// The modules that the module `name` imports, if it is a module of the project.
    fn imported_by(&self, name: &str) -> impl Iterator<Item = &String> {
        self.index
            .get(name)
            .into_iter()
            .flat_map(|&at| self.modules[at].imports.iter().flat_map(|(_, to)| to))
    }

    /// The chain from `target` to a module that a module at `from` may not import, through
    /// modules of no layer, shortest first (`[target]` when `target` is one), with why it breaks
    /// the layers.
    pub fn breach_from<'l>(
        &self,
        layers: &'l Layers,
        from: Place,
        target: &str,
    ) -> Option<(Vec<String>, Breach<'l>)> {
        let breaks = |name: &str| layers.place_of(name).map(|to| layers.breach(from, to));
        match breaks(target) {
            Some(Some(breach)) => return Some((vec![target.to_owned()], breach)),
            Some(None) => return None,
            None => {}
        }
        let mut came_from: HashMap<&str, &str> = HashMap::new();
        let mut queue = VecDeque::from([target]);
        while let Some(current) = queue.pop_front() {
            for next in self.imported_by(current) {
                if next == target || came_from.contains_key(next.as_str()) {
                    continue;
                }
                came_from.insert(next, current);
                match breaks(next) {
                    Some(Some(breach)) => return Some((chain(&came_from, target, next), breach)),
                    Some(None) => {}
                    None => queue.push_back(next),
                }
            }
        }
        None
    }
}

fn chain(came_from: &HashMap<&str, &str>, target: &str, last: &str) -> Vec<String> {
    let mut chain = vec![last.to_owned()];
    let mut at = last;
    while at != target {
        at = came_from[at];
        chain.push(at.to_owned());
    }
    chain.reverse();
    chain
}

/// The modules of the project that an import names: for each name the longest one that is a
/// module of the project, or the name itself inside a top package (a package without a file).
fn targets(
    import: &Import,
    name: &str,
    package: bool,
    known: &[&str],
    tops: &[&str],
) -> Vec<String> {
    let base = if import.level == 0 {
        None
    } else {
        match base_of(name, package, import.level) {
            Some(base) => Some(base),
            None => return Vec::new(),
        }
    };
    let mut found: Vec<String> = Vec::new();
    for imported in &import.names {
        let whole = match (&base, imported.is_empty()) {
            (Some(base), true) => base.clone(),
            (Some(base), false) => format!("{base}.{imported}"),
            (None, _) => imported.clone(),
        };
        let resolved = known
            .iter()
            .filter(|module| within(&whole, module))
            .max_by_key(|module| module.len())
            .map(|module| (*module).to_owned())
            .or_else(|| tops.iter().any(|top| within(&whole, top)).then_some(whole));
        if let Some(resolved) = resolved.filter(|resolved| !found.contains(resolved)) {
            found.push(resolved);
        }
    }
    found
}
