//! The layers of the configuration: a list from the top layer to the bottom one, each one module
//! or several independent modules separated by `|`.

/// Where a module sits: the index of its layer from the top, and of its module in the layer.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Place {
    pub layer: usize,
    pub sibling: usize,
}

pub struct Layers {
    levels: Vec<Vec<String>>,
}

/// Why an import breaks the layers.
#[derive(Clone)]
pub enum Breach<'a> {
    /// The imported module is in a layer above.
    Above { from: &'a str, to: &'a str },
    /// The imported module is another independent module of the same layer.
    Sibling { from: &'a str, to: &'a str },
}

impl Layers {
    pub fn parse(layers: &[String]) -> Self {
        Self {
            levels: layers
                .iter()
                .map(|level| {
                    level
                        .split('|')
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned)
                        .collect::<Vec<String>>()
                })
                .filter(|level| !level.is_empty())
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    /// The first part of the name of each module of the layers: the top packages whose modules
    /// the rule reads.
    pub fn top_packages(&self) -> Vec<&str> {
        let mut tops: Vec<&str> = self
            .levels
            .iter()
            .flatten()
            .map(|name| name.split('.').next().unwrap_or(name))
            .collect();
        tops.sort_unstable();
        tops.dedup();
        tops
    }

    /// The place of `module`, when it is a module of the layers or inside one.
    pub fn place_of(&self, module: &str) -> Option<Place> {
        self.levels.iter().enumerate().find_map(|(layer, level)| {
            level
                .iter()
                .position(|name| within(module, name))
                .map(|sibling| Place { layer, sibling })
        })
    }

    fn name(&self, place: Place) -> &str {
        &self.levels[place.layer][place.sibling]
    }

    /// Why a module at `from` may not import one at `to`, if it may not.
    pub fn breach(&self, from: Place, to: Place) -> Option<Breach<'_>> {
        let names = (self.name(from), self.name(to));
        if to.layer < from.layer {
            Some(Breach::Above {
                from: names.0,
                to: names.1,
            })
        } else if to.layer == from.layer && to.sibling != from.sibling {
            Some(Breach::Sibling {
                from: names.0,
                to: names.1,
            })
        } else {
            None
        }
    }
}

/// Whether `module` is `package` or a module inside it.
pub fn within(module: &str, package: &str) -> bool {
    module
        .strip_prefix(package)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}
