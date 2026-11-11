//! The report as text or as markdown. A value that a line of the history holds is printed inside
//! code in markdown, and runs nothing.

use super::pairs::Reason;
use super::report::Report;
use clap::ValueEnum;
use std::fmt::Write as _;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Text,
    Markdown,
}

pub fn render(report: &Report, format: Format) -> String {
    let mut page = Page {
        out: String::new(),
        format,
    };
    page.title(&format!(
        "botica history report: the last {} days",
        report.days
    ));
    page.sources(report);
    page.caught(report);
    page.reached(report);
    page.not_comparable(report);
    page.quiet(report);
    page.noisy(report);
    page.roles(report);
    page.out
}

struct Page {
    out: String,
    format: Format,
}

impl Page {
    fn title(&mut self, text: &str) {
        match self.format {
            Format::Text => {
                let _ = writeln!(self.out, "{text}");
            }
            Format::Markdown => {
                let _ = writeln!(self.out, "# {text}");
            }
        }
    }

    fn section(&mut self, text: &str) {
        match self.format {
            Format::Text => {
                let _ = write!(self.out, "\n{text}\n");
            }
            Format::Markdown => {
                let _ = write!(self.out, "\n## {text}\n\n");
            }
        }
    }

    fn line(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
    }

    fn item(&mut self, text: &str) {
        match self.format {
            Format::Text => {
                let _ = writeln!(self.out, "  {text}");
            }
            Format::Markdown => {
                let _ = writeln!(self.out, "- {text}");
            }
        }
    }

    /// A value from the history: in code for markdown, so that it is never read as markup.
    fn value(&self, text: &str) -> String {
        match self.format {
            Format::Text => text.to_owned(),
            Format::Markdown => format!("`{}`", text.replace('`', "'")),
        }
    }

    fn sources(&mut self, report: &Report) {
        let counts = &report.counts;
        self.section("Runs read");
        self.item(&format!("{} runs of the local history", counts.local));
        self.item(&format!(
            "{} runs of CI from trusted sources",
            counts.ci_trusted
        ));
        self.item(&format!(
            "{} runs of CI from sources that are not trusted: they feed the paired readings and \
             nothing else",
            counts.ci_untrusted
        ));
        for (kind, count) in &counts.unknown {
            let kind = self.value(kind);
            self.item(&format!(
                "{count} runs of a mode or origin that this report does not know ({kind}): they \
                 enter no reading"
            ));
        }
        if report.skipped_lines > 0 {
            self.item(&format!(
                "{} lines failed their checks (ADR 0019 decision 9.4) and were skipped",
                report.skipped_lines
            ));
        }
        for note in &report.notes {
            let note = self.value(note);
            self.item(&format!("note: {note}"));
        }
    }

    fn caught(&mut self, report: &Report) {
        let paired = &report.paired;
        self.section("Caught before the commit");
        self.line(
            "Findings of a hook run of mode `staged` that the CI run of mode `diff` paired with \
             it does not hold: the rule found them before the commit.",
        );
        if paired.hook_runs == 0 {
            self.line(
                "No hook run of mode `staged` in the window. The hooks of the mirror run in mode \
                 `paths`; a hook of `botica lint --staged --record` with `pass_filenames: false` \
                 feeds this reading.",
            );
        } else if paired.paired_hooks == 0 {
            self.line(&format!(
                "{} hook runs of mode `staged`, and none has a CI run of mode `diff` on its \
                 branch after it. Download the runs of CI with `botica history fetch` and pass \
                 them with `--from`.",
                paired.hook_runs
            ));
        } else {
            self.line(&format!(
                "{} of {} hook runs have a CI run to pair with.",
                paired.paired_hooks, paired.hook_runs
            ));
            self.counts(&paired.caught.clone());
        }
    }

    fn reached(&mut self, report: &Report) {
        let paired = &report.paired;
        self.section("Reached CI");
        self.line(
            "Findings of a CI run of mode `diff` that no hook run of mode `staged` before it \
             held: a gap of the local layer, or a commit made with `--no-verify`. The history \
             cannot tell them apart.",
        );
        if paired.ci_runs == 0 {
            self.line(
                "No CI run of mode `diff` in the window. A CI job of `botica lint --diff <base> \
                 --record` feeds this reading.",
            );
        } else if paired.paired_ci == 0 {
            self.line(&format!(
                "{} CI runs of mode `diff`, and none has a hook run of mode `staged` before it.",
                paired.ci_runs
            ));
        } else {
            self.line(&format!(
                "{} of {} CI runs have a hook run before them.",
                paired.paired_ci, paired.ci_runs
            ));
            self.counts(&paired.reached.clone());
        }
    }

    fn counts(&mut self, found: &std::collections::BTreeMap<String, u64>) {
        if found.is_empty() {
            self.line("No finding.");
        }
        for (slug, count) in found {
            let slug = self.value(slug);
            self.item(&format!("{slug}: {count}"));
        }
    }

    fn not_comparable(&mut self, report: &Report) {
        if report.paired.heads_unasked > 0 {
            self.section("Pairing by time");
            self.line(&format!(
                "{} heads of CI runs were past the limit of the questions that the report asks \
                 git, so their runs were paired by time.",
                report.paired.heads_unasked
            ));
        }
        let rules = &report.paired.not_comparable;
        if rules.is_empty() {
            return;
        }
        self.section("Rules that are not comparable");
        self.line(
            "A rule takes part in a pairing only when both runs record the same hash for it, \
             and a hash of `null` is not comparable. The findings of these rules are left out of \
             the two readings above.",
        );
        for (slug, reasons) in rules {
            let why: Vec<&str> = reasons
                .iter()
                .map(|reason| match reason {
                    Reason::Unhashed => "a run records no hash for it",
                    Reason::Changed => "the hash differs between the runs",
                })
                .collect();
            let slug = self.value(slug);
            self.item(&format!("{slug}: {}", why.join("; ")));
        }
    }

    fn quiet(&mut self, report: &Report) {
        self.section("Rules with no findings");
        self.line(&format!(
            "Rules that ran in at least one of the {} runs of the window and had no finding, no \
             suppressed finding and no baselined finding in any of them. Zero can mean that the \
             rule works: the agent may follow it already because `AGENTS.md` lists it. A rule \
             with no findings is a candidate for review, and a person decides to retire it.",
            report.counts.rule_runs
        ));
        if report.counts.rule_runs == 0 {
            self.line("No run of a trusted source in the window.");
        } else if report.quiet.is_empty() {
            self.line("None.");
        }
        for rule in &report.quiet {
            let slug = self.value(&rule.slug);
            self.item(&format!("{slug}: no finding in {} runs", rule.runs));
        }
    }

    fn noisy(&mut self, report: &Report) {
        self.section("Rules with many suppressions");
        let noise = report.noise;
        self.line(&format!(
            "Rules with at least {} suppressed findings, which are at least {:.0}% of their \
             findings (suppressed over hits plus suppressed).",
            noise.min_suppressed,
            noise.min_share * 100.0
        ));
        if report.noisy.is_empty() {
            self.line("None.");
        }
        for rule in &report.noisy {
            let slug = self.value(&rule.slug);
            self.item(&format!(
                "{slug}: {} suppressed, {} reported ({:.0}%)",
                rule.suppressed,
                rule.hits,
                rule.share() * 100.0
            ));
        }
    }

    fn roles(&mut self, report: &Report) {
        if report.roles.is_empty() {
            return;
        }
        self.section("Findings of the roles that refute did not confirm");
        for role in &report.roles {
            let slug = self.value(&role.slug);
            self.item(&format!(
                "{slug}: {} of {} judged findings, {:.0}%",
                role.dismissed,
                role.judged,
                role.dismissed as f64 / role.judged as f64 * 100.0
            ));
        }
    }
}
