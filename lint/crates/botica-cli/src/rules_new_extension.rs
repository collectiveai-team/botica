//! What `rules new --kind extension` writes besides the files of every rule: the implementation of
//! the runtime and the script that it runs.

use clap::ValueEnum;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NewRuntime {
    Python,
    Node,
    Command,
}

/// The file of the runtime `command` that the catalog does not read: the command names it.
pub const COMMAND_SCRIPT: &str = "check.sh";

/// The script of an extension rule: its file name in the folder of the rule and its text.
pub struct Script {
    pub name: String,
    pub text: String,
}

/// The lines of the `[[implementations]]` entry of an extension, after `kind = "extension"`.
pub fn implementation(runtime: NewRuntime, language: &str, slug: &str, root: &str) -> String {
    let script = script_name(runtime, slug);
    match runtime {
        NewRuntime::Python => {
            format!("runtime = \"python\"\nlanguages = [\"{language}\"]\nfile = \"{script}\"\n")
        }
        NewRuntime::Node => {
            format!("runtime = \"node\"\nlanguages = [\"{language}\"]\nfile = \"{script}\"\n")
        }
        NewRuntime::Command => format!(
            "runtime = \"command\"\nlanguages = [\"{language}\"]\n\
             command = [\"sh\", \"{root}/{script}\"]\n"
        ),
    }
}

fn script_name(runtime: NewRuntime, slug: &str) -> String {
    match runtime {
        NewRuntime::Python => format!("{slug}.py"),
        NewRuntime::Node => format!("{slug}.mjs"),
        NewRuntime::Command => COMMAND_SCRIPT.to_owned(),
    }
}

pub fn script(runtime: NewRuntime, id: &str, language: &str, slug: &str) -> Script {
    let text = match runtime {
        NewRuntime::Python => python(id, language),
        NewRuntime::Node => node(id, language),
        NewRuntime::Command => COMMAND.to_owned(),
    };
    Script {
        name: script_name(runtime, slug),
        text,
    }
}

fn python(id: &str, language: &str) -> String {
    format!(
        "# /// script\n# requires-python = \">=3.10\"\n\
         # dependencies = [\"botica[sdk]=={}\"]\n# ///\n\
         from botica.sdk import rule, serve\n\n\n\
         @rule(\"{id}\", [\"{language}\"])\ndef check(path, text, options):\n    \
         # TODO: yield (start, end, message) for each finding; start and end are byte offsets,\n    \
         # and options holds the options of the rule\n    \
         return []\n\n\n\
         if __name__ == \"__main__\":\n    serve()\n",
        env!("CARGO_PKG_VERSION")
    )
}

fn node(id: &str, language: &str) -> String {
    format!(
        "import {{ createInterface }} from \"node:readline\";\n\n\
         const RULE = \"{id}\";\n\n\
         // TODO: return {{ start, end, message }} for each finding; start and end are byte offsets\n\
         function check(path, text) {{\n  return [];\n}}\n\n\
         const reply = (id, result) =>\n  \
         console.log(JSON.stringify({{ jsonrpc: \"2.0\", id, result }}));\n\n\
         for await (const line of createInterface({{ input: process.stdin }})) {{\n  \
         const request = JSON.parse(line);\n  \
         if (request.method === \"initialize\") {{\n    \
         reply(request.id, {{ protocolVersion: 1, rules: [{{ id: RULE, languages: [\"{language}\"] }}] }});\n  \
         }} else if (request.method === \"lint/files\") {{\n    \
         const diagnostics = request.params.files.flatMap((file) =>\n      \
         check(file.path, file.text).map((found) => ({{\n        \
         path: file.path,\n        rule: RULE,\n        severity: \"warning\",\n        ...found,\n      }})),\n    );\n    \
         reply(request.id, {{ diagnostics }});\n  \
         }} else if (request.method === \"shutdown\") {{\n    \
         reply(request.id, null);\n    break;\n  }}\n}}\n\n\
         process.stdin.destroy();\n"
    )
}

const COMMAND: &str =
    "# TODO: speak the extension protocol of botica on stdin and stdout: newline-delimited\n\
# JSON-RPC 2.0, with the methods `initialize`, `lint/files`, `lint/project` and `shutdown`.\n";
