use std::path::Path;

/// Whether the `.gitignore` of `root` or of a folder on the way to `relative`, a path from `root`
/// that may not exist, leaves it out. The `.gitignore` nearest to it decides.
pub fn is_gitignored(root: &Path, relative: &str) -> bool {
    let mut dir = root.to_path_buf();
    let mut decided = false;
    let parts: Vec<&str> = relative
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in parts
        .iter()
        .take(parts.len().saturating_sub(1))
        .copied()
        .chain([""])
    {
        let file = dir.join(".gitignore");
        if file.is_file() {
            let (gitignore, _) = ignore::gitignore::Gitignore::new(&file);
            match gitignore.matched_path_or_any_parents(root.join(relative), false) {
                ignore::Match::Ignore(_) => decided = true,
                ignore::Match::Whitelist(_) => decided = false,
                ignore::Match::None => {}
            }
        }
        dir.push(part);
    }
    decided
}
