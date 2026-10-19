//! The settings classes of pydantic-settings in a Python file: the classes, their bases, their
//! annotated fields, `model_config` and the aliases of `Field(...)`.

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::Node;
use ast_grep_language::{LanguageExt, SupportLang};

use super::settings::{Alias, Class, Config, Field, Text};

type PyNode<'r> = Node<'r, StrDoc<SupportLang>>;

/// The name of the base class of pydantic-settings; a file that never names it nor a class the
/// project found is not parsed.
pub const ROOT: &str = "BaseSettings";

/// Every class that `text` defines, settings class or not.
pub fn classes(path: &str, text: &str) -> Vec<Class> {
    let grep = SupportLang::Python.ast_grep(text);
    grep.root()
        .dfs()
        .filter(|node| node.kind() == "class_definition")
        .filter_map(|node| class(path, &node))
        .collect()
}

fn class(path: &str, node: &PyNode<'_>) -> Option<Class> {
    let name = node.field("name")?.text().into_owned();
    let bases: Vec<String> = node
        .field("superclasses")
        .map(|list| {
            list.children()
                .filter(|base| matches!(&*base.kind(), "identifier" | "attribute"))
                .map(|base| last_part(&base.text()).to_owned())
                .collect()
        })
        .unwrap_or_default();
    let mut config = Config::default();
    let mut fields = Vec::new();
    for statement in node.field("body")?.children() {
        match &*statement.kind() {
            "class_definition" => {
                if statement
                    .field("name")
                    .is_some_and(|n| n.text() == "Config")
                {
                    config.opaque = true;
                }
            }
            "expression_statement" => {
                let Some(assignment) = statement.children().find(|c| c.kind() == "assignment")
                else {
                    continue;
                };
                member(&assignment, &mut config, &mut fields);
            }
            _ => {}
        }
    }
    Some(Class {
        path: path.to_owned(),
        extends_root: bases.iter().any(|base| base == ROOT),
        name,
        bases,
        config,
        fields,
    })
}

/// Adds what one assignment of a class body says: the configuration, or a field.
fn member(assignment: &PyNode<'_>, config: &mut Config, fields: &mut Vec<Field>) {
    let Some(left) = assignment
        .field("left")
        .filter(|l| l.kind() == "identifier")
    else {
        return;
    };
    let name = left.text();
    let annotation = assignment.field("type");
    let value = assignment.field("right");
    if name == "model_config" {
        if let Some(value) = value {
            *config = model_config(&value);
        }
        return;
    }
    let Some(annotation) = annotation else {
        return;
    };
    let annotation_text = annotation.text();
    if name.starts_with('_')
        || last_part(annotation_text.split('[').next().unwrap_or("")) == "ClassVar"
    {
        return;
    }
    let alias = [value.as_ref(), Some(&annotation)]
        .into_iter()
        .flatten()
        .find_map(field_alias)
        .unwrap_or(Alias::None);
    let range = left.range();
    fields.push(Field {
        name: name.into_owned(),
        start: range.start,
        end: range.end,
        alias,
        type_names: annotation
            .dfs()
            .filter(|n| n.kind() == "identifier")
            .map(|n| n.text().into_owned())
            .collect(),
    });
}

/// The keys of `SettingsConfigDict(...)` or of a dict literal.
fn model_config(value: &PyNode<'_>) -> Config {
    let mut config = Config::default();
    let pairs: Vec<(String, PyNode<'_>)> = match &*value.kind() {
        "call"
            if value.field("function").is_some_and(|function| {
                matches!(
                    last_part(&function.text()),
                    "SettingsConfigDict" | "ConfigDict" | "dict"
                )
            }) =>
        {
            value
                .field("arguments")
                .map(|arguments| keywords(&arguments))
                .unwrap_or_default()
        }
        "dictionary" => value
            .children()
            .filter(|pair| pair.kind() == "pair")
            .filter_map(|pair| Some((literal(&pair.field("key")?)?, pair.field("value")?)))
            .collect(),
        _ => {
            config.opaque = true;
            return config;
        }
    };
    for (key, value) in pairs {
        match key.as_str() {
            "env_prefix" => config.env_prefix = Some(text(&value)),
            "env_nested_delimiter" if value.kind() != "none" => {
                config.nested_delimiter = Some(text(&value));
            }
            "case_sensitive" => config.case_sensitive = Some(value.kind() == "true"),
            "alias_generator" => config.opaque = true,
            _ => {}
        }
    }
    config
}

/// The alias of the first `Field(...)` call in `node`, if it names one.
fn field_alias(node: &PyNode<'_>) -> Option<Alias> {
    let call = node.dfs().find(|n| {
        n.kind() == "call"
            && n.field("function")
                .is_some_and(|f| last_part(&f.text()) == "Field")
    })?;
    let arguments = keywords(&call.field("arguments")?);
    let value = ["validation_alias", "alias"]
        .into_iter()
        .find_map(|key| arguments.iter().find(|(name, _)| name == key))
        .map(|(_, value)| value)?;
    if let Some(name) = literal(value) {
        return Some(Alias::Name(name));
    }
    let function = value
        .field("function")
        .map(|f| last_part(&f.text()).to_owned());
    let names: Vec<String> = value
        .dfs()
        .filter(|n| n.kind() == "string")
        .filter_map(|n| literal(&n))
        .collect();
    Some(match function.as_deref() {
        Some("AliasChoices" | "AliasPath") => Alias::Choices(names),
        _ => Alias::Unknown,
    })
}

/// The keyword arguments of an argument list, by name.
fn keywords<'r>(arguments: &PyNode<'r>) -> Vec<(String, PyNode<'r>)> {
    arguments
        .children()
        .filter(|argument| argument.kind() == "keyword_argument")
        .filter_map(|argument| {
            Some((
                argument.field("name")?.text().into_owned(),
                argument.field("value")?,
            ))
        })
        .collect()
}

fn text(node: &PyNode<'_>) -> Text {
    literal(node).map_or(Text::Unknown, Text::Known)
}

/// The value of a plain string literal: no f-string, no implicit concatenation.
fn literal(node: &PyNode<'_>) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    let source = node.text();
    let body = source.trim_start_matches(['r', 'R', 'u', 'U', 'b', 'B']);
    ["\"\"\"", "'''", "\"", "'"].into_iter().find_map(|quote| {
        body.strip_prefix(quote)
            .and_then(|inside| inside.strip_suffix(quote))
            .map(str::to_owned)
    })
}

fn last_part(dotted: &str) -> &str {
    dotted.rsplit('.').next().unwrap_or(dotted).trim()
}
