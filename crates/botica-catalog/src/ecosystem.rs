/// A family of languages that share a manifest and a way of laying a project out. The rules about
/// layout apply to the code of a project, and where that code lives depends on its ecosystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ecosystem {
    Python,
    /// JavaScript and TypeScript.
    Js,
    Rust,
    Go,
    /// Java, Kotlin and Scala, built with Maven or Gradle.
    Jvm,
}

impl Ecosystem {
    pub const ALL: [Self; 5] = [Self::Python, Self::Js, Self::Rust, Self::Go, Self::Jvm];

    pub fn name(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Js => "js",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Jvm => "jvm",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|ecosystem| ecosystem.name() == name)
    }
}
