use ast_grep_language::{LanguageExt, SupportLang};
use botica_astgrep::{AstGrepRule, AstGrepSet};
use botica_core::{Engine, FileSizeGuard, NativeRule, NoUtils};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const PY_RULES: [&str; 10] = [
    include_str!("../../../rules/fastapi/api-schemas-extra-forbid/python.yml"),
    include_str!("../../../rules/architecture/arch-database-package/python.yml"),
    include_str!("../../../rules/process/cli-typed-framework/python.yml"),
    include_str!("../../../rules/logging/log-get-logger/python.yml"),
    include_str!("../../../rules/logging/log-no-print/python.yml"),
    include_str!("../../../rules/data-models/no-dict-alias/python.yml"),
    include_str!("../../../rules/data-models/no-dict-call-return/python.yml"),
    include_str!("../../../rules/data-models/no-dict-literal-return/python.yml"),
    include_str!("../../../rules/data-models/no-dict-return-annotation/python.yml"),
    include_str!("../../../rules/architecture/settings-module/python.yml"),
];
const TS_RULE: &str = include_str!("../../../rules/logging/log-no-print/typescript.yml");

const PY_BLOCK: &str = r#"
import logging
from dataclasses import dataclass


@dataclass
class Item{n}:
    name: str
    size: int


def load_{n}(path: str) -> dict:
    data = {{"path": path, "n": {n}}}
    print("loading", path)
    return data


def summarize_{n}(items: list[Item{n}]) -> int:
    total = 0
    for item in items:
        if item.size > 10:
            total += item.size
        else:
            total -= 1
    logger = logging.getLogger(__name__)
    return total
"#;

const TS_BLOCK: &str = r#"
export interface Item{n} {{ name: string; size: number }}

export function load{n}(path: string): Item{n} {{
  console.log("loading", path);
  try {{
    return {{ name: path, size: {n} }};
  }} catch (error) {{
    console.log(error);
    throw error;
  }}
}}

export const sum{n} = (items: Item{n}[]): number =>
  items.reduce((acc, item) => acc + item.size, 0);
"#;

fn buffer(block: &str, lines: usize) -> String {
    let mut text = String::new();
    let mut n = 0;
    while text.lines().count() < lines {
        text.push_str(
            &block
                .replace("{n}", &n.to_string())
                .replace("{{", "{")
                .replace("}}", "}"),
        );
        n += 1;
    }
    text
}

fn engine(py: bool, ts: bool) -> Engine {
    let native: Vec<Box<dyn NativeRule>> = vec![
        Box::new(FileSizeGuard::default()),
        Box::new(NoUtils::default()),
    ];
    let mut rules = Vec::new();
    if py {
        for yaml in PY_RULES {
            rules.extend(AstGrepRule::from_yaml(yaml).unwrap());
        }
    }
    if ts {
        rules.extend(AstGrepRule::from_yaml(TS_RULE).unwrap());
    }
    Engine::new(native, AstGrepSet::new(rules))
}

fn lint_source(c: &mut Criterion) {
    let py2000 = buffer(PY_BLOCK, 2000);
    let py100 = buffer(PY_BLOCK, 100);
    let ts2000 = buffer(TS_BLOCK, 2000);

    let native_only = engine(false, false);
    let full = engine(true, true);

    let mut group = c.benchmark_group("lint_source");
    group.bench_function("py_2000_lines/native_only", |b| {
        b.iter(|| native_only.lint_source("pkg/mod.py", black_box(&py2000)));
    });
    group.bench_function("py_2000_lines/native+10_astgrep", |b| {
        b.iter(|| full.lint_source("pkg/mod.py", black_box(&py2000)));
    });
    group.bench_function("py_100_lines/native+10_astgrep", |b| {
        b.iter(|| full.lint_source("pkg/mod.py", black_box(&py100)));
    });
    group.bench_function("ts_2000_lines/native+1_astgrep", |b| {
        b.iter(|| full.lint_source("src/mod.ts", black_box(&ts2000)));
    });
    group.finish();

    // One keystroke in the middle of the buffer, then back: each iteration is two edits.
    let mut typing = c.benchmark_group("lsp_keystroke_x2");
    for (name, path, text) in [
        ("py_2000_lines", "pkg/mod.py", &py2000),
        ("ts_2000_lines", "src/mod.ts", &ts2000),
    ] {
        let at = text[text.len() / 2..].find('\n').unwrap() + text.len() / 2 + 1;
        let mut buffer = full.astgrep().open(path.as_ref(), text).unwrap();
        typing.bench_function(format!("{name}/reparse_only"), |b| {
            b.iter(|| {
                buffer.edit(at, 0, "#").unwrap();
                buffer.edit(at, 1, "").unwrap();
            });
        });
        typing.bench_function(format!("{name}/reparse+rules"), |b| {
            b.iter(|| {
                buffer.edit(at, 0, "#").unwrap();
                black_box(full.astgrep().check_buffer(&buffer));
                buffer.edit(at, 1, "").unwrap();
                black_box(full.astgrep().check_buffer(&buffer));
            });
        });
    }
    typing.finish();

    let mut parse = c.benchmark_group("parse_only");
    parse.bench_function("py_2000_lines", |b| {
        b.iter(|| SupportLang::Python.ast_grep(black_box(&py2000)));
    });
    parse.bench_function("ts_2000_lines", |b| {
        b.iter(|| SupportLang::TypeScript.ast_grep(black_box(&ts2000)));
    });
    parse.finish();

    c.bench_function("compile/10_py_rules+1_ts_rule", |b| {
        b.iter(|| engine(true, true));
    });
}

criterion_group!(benches, lint_source);
criterion_main!(benches);
