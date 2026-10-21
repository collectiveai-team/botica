//! `cognitive-complexity`: a function whose cognitive complexity, as `botica-complexity` measures
//! it, is above the limit.

use std::path::Path;

use ast_grep_language::{Language, SupportLang};

use crate::diagnostic::{Diagnostic, Measure, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};

pub const COGNITIVE_COMPLEXITY: &str = "cognitive-complexity";

#[derive(Debug)]
pub struct CognitiveComplexity {
    pub max: u32,
}

impl Default for CognitiveComplexity {
    fn default() -> Self {
        Self { max: 15 }
    }
}

impl NativeRule for CognitiveComplexity {
    fn id(&self) -> &'static str {
        COGNITIVE_COMPLEXITY
    }

    fn needs(&self) -> Needs {
        Needs::Text
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let (Some(text), Some(language)) =
            (doc.text(), SupportLang::from_path(Path::new(doc.path)))
        else {
            return;
        };
        for function in botica_complexity::measure(language, text) {
            if function.complexity <= self.max {
                continue;
            }
            let (line, column) = doc.position(function.start);
            let (end_line, end_column) = doc.position(function.end);
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: COGNITIVE_COMPLEXITY.to_owned(),
                severity: Severity::Error,
                start: function.start,
                end: function.end,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                message: format!(
                    "`{}` has a cognitive complexity of {} (the limit is {}): move nested \
                     conditions and loops into functions of their own",
                    function.name, function.complexity, self.max
                ),
                measure: Some(Measure {
                    name: function.name,
                    value: function.complexity,
                }),
            });
        }
    }
}
