use std::collections::BTreeMap;

use anyhow::{ensure, Context, Result};
use botica_core::review::ReviewChange;

use super::Git;

type Counts = (Option<u64>, Option<u64>);
type Sizes = BTreeMap<Vec<String>, Counts>;

const OPTIONS: [&str; 7] = [
    "diff",
    "--no-ext-diff",
    "--no-textconv",
    "--ignore-submodules=none",
    "--no-abbrev",
    "-M",
    "-z",
];

fn count(value: &[u8]) -> Result<Option<u64>> {
    if value == b"-" {
        Ok(None)
    } else {
        Ok(Some(std::str::from_utf8(value)?.parse()?))
    }
}

fn numstats(bytes: &[u8]) -> Result<Sizes> {
    let mut sizes = BTreeMap::new();
    let mut records = bytes.split(|&b| b == 0);
    while let Some(record) = records.next().filter(|r| !r.is_empty()) {
        let mut fields = record.splitn(3, |&b| b == b'\t');
        let added = fields.next().context("missing added count")?;
        let deleted = fields.next().context("missing deleted count")?;
        let path = fields.next().context("missing numstat path")?;
        let paths = if path.is_empty() {
            vec![
                records.next().context("missing rename source")?,
                records.next().context("missing rename destination")?,
            ]
        } else {
            vec![path]
        };
        let paths = paths
            .into_iter()
            .map(|path| std::str::from_utf8(path).map(str::to_owned))
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("non-UTF-8 changed path")?;
        sizes.insert(paths, (count(added)?, count(deleted)?));
    }
    Ok(sizes)
}

fn kind(modes: [&str; 2], added: Option<u64>) -> &'static str {
    if modes.contains(&"160000") {
        "submodule"
    } else if modes.contains(&"120000") {
        "symbolic-link"
    } else if added.is_none() {
        "binary"
    } else {
        "text"
    }
}

impl Git {
    fn text_blob(&self, id: &str, kind: &str) -> Result<Option<String>> {
        if kind != "text" || id.bytes().all(|b| b == b'0') {
            return Ok(None);
        }
        let size: u64 = self.text(&["cat-file", "-s", id])?.parse()?;
        ensure!(
            size <= 16 * 1024 * 1024,
            "text evidence exceeds the supported 16 MiB per-blob limit"
        );
        Ok(Some(
            String::from_utf8(self.read(&["cat-file", "blob", id])?)
                .context("text detector input is not UTF-8")?,
        ))
    }

    fn raw_changes(&self, bytes: &[u8], mut sizes: Sizes) -> Result<Vec<ReviewChange>> {
        let mut records = bytes.split(|&b| b == 0);
        let mut changes = Vec::new();
        while let Some(header) = records.next().filter(|r| !r.is_empty()) {
            let fields: Vec<_> = std::str::from_utf8(header)?.split(' ').collect();
            ensure!(
                fields.len() == 5 && fields[0].starts_with(':'),
                "invalid raw diff metadata"
            );
            let mut paths =
                vec![std::str::from_utf8(records.next().context("missing raw path")?)?.to_owned()];
            if fields[4].starts_with(['R', 'C']) {
                paths.push(
                    std::str::from_utf8(records.next().context("missing rename destination")?)?
                        .to_owned(),
                );
            }
            let (mut added, mut deleted) = sizes
                .remove(&paths)
                .context("raw diff and numstat disagree")?;
            let kind = kind([fields[0].trim_start_matches(':'), fields[1]], added);
            if kind != "text" {
                added = None;
                deleted = None;
            }
            changes.push(ReviewChange {
                paths,
                added,
                deleted,
                kind: kind.to_owned(),
                status: fields[4].to_owned(),
                before: self.text_blob(fields[2], kind)?,
                after: self.text_blob(fields[3], kind)?,
            });
        }
        ensure!(sizes.is_empty(), "unmatched numstat entries");
        Ok(changes)
    }

    pub fn changes(&self, from: &str, to: &str) -> Result<Vec<ReviewChange>> {
        let mut args = OPTIONS.to_vec();
        args.extend(["--numstat", from, to, "--"]);
        let sizes = numstats(&self.read(&args)?)?;
        let mut args = OPTIONS.to_vec();
        args.extend(["--raw", from, to, "--"]);
        self.raw_changes(&self.read(&args)?, sizes)
    }
}
