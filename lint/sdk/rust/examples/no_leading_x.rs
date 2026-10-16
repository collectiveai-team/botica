//! A rule that flags each line that starts with `x`. Build it with `cargo build --example
//! no_leading_x -p botica-ext-sdk` and name the binary in the `command` of an extension rule.

use botica_ext_sdk::{serve, Finding, Rule};

fn main() -> std::io::Result<()> {
    let rules = [Rule::new("acme/no-leading-x", &["python"], |_path, text| {
        let mut offset = 0;
        let mut found = Vec::new();
        for line in text.split_inclusive('\n') {
            if line.starts_with('x') {
                found.push(Finding::new(offset, offset + 1, "a line starts with x"));
            }
            offset += line.len();
        }
        found
    })
    .severity("error")];
    serve(&rules)
}
