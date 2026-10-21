use anyhow::Result;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::AstGrep;
use ast_grep_language::{Language, LanguageExt, SupportLang};
use std::path::Path;

/// Whether this build carries the grammar for `language`. ast-grep panics when asked for a
/// grammar whose feature is off, so callers must check first.
pub fn compiled(language: SupportLang) -> bool {
    use SupportLang as L;
    match language {
        L::Bash
        | L::C
        | L::Css
        | L::Go
        | L::Hcl
        | L::Html
        | L::Java
        | L::JavaScript
        | L::Json
        | L::Lua
        | L::Python
        | L::Rust
        | L::Tsx
        | L::TypeScript
        | L::Yaml => true,
        L::Cpp => cfg!(feature = "lang-cpp"),
        L::CSharp => cfg!(feature = "lang-csharp"),
        L::Dart => cfg!(feature = "lang-dart"),
        L::Elixir => cfg!(feature = "lang-elixir"),
        L::Haskell => cfg!(feature = "lang-haskell"),
        L::Kotlin => cfg!(feature = "lang-kotlin"),
        L::Markdown => cfg!(feature = "lang-markdown"),
        L::Nix => cfg!(feature = "lang-nix"),
        L::Php => cfg!(feature = "lang-php"),
        L::Ruby => cfg!(feature = "lang-ruby"),
        L::Scala => cfg!(feature = "lang-scala"),
        L::Solidity => cfg!(feature = "lang-solidity"),
        L::Swift => cfg!(feature = "lang-swift"),
    }
}

/// The language a file is linted as, if its grammar is in this build.
pub(crate) fn language_of(path: &Path) -> Option<SupportLang> {
    SupportLang::from_path(path).filter(|&language| compiled(language))
}

/// Rejects rules for languages this build lacks, before ast-grep compiles their patterns.
pub(crate) fn check_languages(yaml: &str) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Head {
        language: Option<String>,
    }
    for document in serde_yaml::Deserializer::from_str(yaml) {
        let Ok(head) = <Head as serde::Deserialize>::deserialize(document) else {
            // ast-grep reports malformed rules itself. Stop here: after an error the
            // deserializer does not advance, so `continue` would loop forever.
            break;
        };
        let Some(name) = head.language else { continue };
        if let Ok(language) = name.parse::<SupportLang>() {
            anyhow::ensure!(
                compiled(language),
                "rule language `{name}` is not compiled into this botica build \
                 (enable the botica-astgrep feature for it, or `all-languages`)"
            );
        }
    }
    Ok(())
}

thread_local! {
    static PARSERS: std::cell::RefCell<Vec<(SupportLang, tree_sitter::Parser)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// ast-grep 0.41 builds a `Parser` and calls `set_language` for every file; this keeps one
/// per thread and language.
pub(crate) fn parse_reusing(
    lang: SupportLang,
    text: &str,
    ranges: &[tree_sitter::Range],
) -> Option<AstGrep<StrDoc<SupportLang>>> {
    let tree = PARSERS.with(|cell| {
        let mut parsers = cell.borrow_mut();
        let position = match parsers.iter().position(|(l, _)| *l == lang) {
            Some(position) => position,
            None => {
                let mut parser = tree_sitter::Parser::new();
                parser.set_language(&lang.get_ts_language()).ok()?;
                parsers.push((lang, parser));
                parsers.len() - 1
            }
        };
        let parser = &mut parsers[position].1;
        if ranges.is_empty() {
            return parser.parse(text.as_bytes(), None);
        }
        parser.set_included_ranges(ranges).ok()?;
        let tree = parser.parse(text.as_bytes(), None);
        parser.set_included_ranges(&[]).ok()?;
        tree
    })?;
    Some(AstGrep::doc(StrDoc {
        src: text.to_owned(),
        lang,
        tree,
    }))
}
