//! What an extension implementation names: the script of the runtimes `python` and `node`, the
//! command of the runtime `command`.

use crate::model::{Implementation, Kind, Runtime};

/// The extensions of the script that a runtime runs.
fn script_extensions(runtime: Runtime) -> &'static [&'static str] {
    match runtime {
        Runtime::Python => &["py"],
        Runtime::Node => &["js", "mjs"],
        Runtime::Command => &[],
    }
}

fn word(runtime: Runtime) -> &'static str {
    match runtime {
        Runtime::Python => "python",
        Runtime::Node => "node",
        Runtime::Command => "command",
    }
}

/// A file of the folder of the rule: a name with a stem, and no folder in front of it.
fn is_plain_file(name: &str) -> bool {
    !name.contains('/')
        && !name.contains('\\')
        && name
            .rsplit_once('.')
            .is_some_and(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
}

/// The problems of the fields that name what an implementation runs.
pub(crate) fn check_extension(
    index: usize,
    implementation: &Implementation,
    add: &mut impl FnMut(String),
) {
    let runtime = implementation.runtime;
    if implementation.command.is_some() && runtime != Some(Runtime::Command) {
        add(format!(
            "implementation {index}: `command` applies to the runtime `command` only"
        ));
    }
    if implementation.kind != Kind::Extension {
        return;
    }
    match runtime {
        Some(Runtime::Command) if implementation.file.is_some() => add(format!(
            "implementation {index}: the runtime `command` takes a `command` and no `file`"
        )),
        Some(Runtime::Command) => check_command(index, implementation, add),
        Some(script @ (Runtime::Python | Runtime::Node)) => {
            check_script(index, implementation, script, add);
        }
        None => {}
    }
}

fn check_command(index: usize, implementation: &Implementation, add: &mut impl FnMut(String)) {
    let has_program = implementation
        .command
        .as_ref()
        .and_then(|argv| argv.first())
        .is_some_and(|program| !program.is_empty());
    if !has_program {
        add(format!(
            "implementation {index}: an extension with the runtime `command` needs a `command` with a program, as `command = [\"sh\", \"check.sh\"]`"
        ));
    }
}

fn check_script(
    index: usize,
    implementation: &Implementation,
    runtime: Runtime,
    add: &mut impl FnMut(String),
) {
    let runtime_word = word(runtime);
    let Some(name) = &implementation.file else {
        add(format!(
            "implementation {index}: an extension with the runtime `{runtime_word}` must name its script with `file`"
        ));
        return;
    };
    if !is_plain_file(name) {
        add(format!(
            "implementation {index}: file `{name}` must be a file of the folder of the rule"
        ));
        return;
    }
    let extension = name.rsplit_once('.').map_or("", |(_, extension)| extension);
    let wanted = script_extensions(runtime);
    if !wanted.contains(&extension) {
        let shown: Vec<String> = wanted.iter().map(|e| format!("`.{e}`")).collect();
        add(format!(
            "implementation {index}: the runtime `{runtime_word}` runs a {} script, not `{name}`",
            shown.join(" or ")
        ));
    }
}
