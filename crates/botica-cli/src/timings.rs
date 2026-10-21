use std::time::Duration;

use botica_core::RuleTime;

use crate::extension::ExtensionTime;

fn millis(nanos: u64) -> f64 {
    nanos as f64 / 1e6
}

fn wall(time: Duration) -> f64 {
    time.as_secs_f64() * 1e3
}

/// One row per rule, the slowest first, on stderr so that stdout stays what the format says.
pub fn print_rules(mut times: Vec<RuleTime>) {
    times.sort_by(|a, b| b.nanos.cmp(&a.nanos).then_with(|| a.rule.cmp(&b.rule)));
    eprintln!(
        "[timing] rules: CPU time summed over all threads, so it can exceed the wall time; \
         for ast-grep rules only the matching, not the parse the rules share"
    );
    for time in times {
        eprintln!(
            "[timing] rule {:<24} {:<12} {:>9.2} ms {:>7} files",
            time.rule,
            time.kind.as_str(),
            millis(time.nanos),
            time.files
        );
    }
}

pub fn print_extension(command: &str, time: &ExtensionTime) {
    eprintln!(
        "[timing] extension `{command}`: start {:.2} ms, lint {:.2} ms, shutdown {:.2} ms, total {:.2} ms",
        wall(time.start),
        wall(time.lint),
        wall(time.shutdown),
        wall(time.total())
    );
}
