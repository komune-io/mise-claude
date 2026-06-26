//! Discover skills inside a checked-out repository tree, replicating the
//! skills.sh layout convention.

use std::path::{Path, PathBuf};

/// One discovered skill: its name (directory basename) and directory path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDir {
    pub name: String,
    pub path: PathBuf,
}

/// Discover every skill in `root`.
///
/// Rules:
/// - A root `SKILL.md` means the whole repo is one skill. The skill is named
///   after `repo_name` (the caller supplies the repository name, because the
///   checkout directory is typically a commit SHA and therefore meaningless).
/// - Otherwise, every directory containing a `SKILL.md` is a skill named
///   after its directory basename. A top-level `template/` directory is
///   excluded — it is scaffolding, not an installable skill. Discovery does
///   not descend into a directory once its own `SKILL.md` is found.
///
/// Result is sorted by name for determinism.
pub fn discover_all(root: &Path, repo_name: &str) -> Vec<SkillDir> {
    if root.join("SKILL.md").is_file() {
        return vec![SkillDir {
            name: repo_name.to_string(),
            path: root.to_path_buf(),
        }];
    }

    let mut found = Vec::new();
    collect(root, root, &mut found);
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Find a single skill by name. The name is matched against the directory
/// basename of each skill directory; for single-skill repos the name matches
/// the repository name (`repo_name`). Returns `None` if no matching skill
/// exists.
pub fn find_one(root: &Path, repo_name: &str, name: &str) -> Option<SkillDir> {
    discover_all(root, repo_name)
        .into_iter()
        .find(|s| s.name == name)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<SkillDir>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Exclude a top-level `template/` scaffolding directory.
        if dir == root && path.file_name().is_some_and(|n| n == "template") {
            continue;
        }
        if path.join("SKILL.md").is_file() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push(SkillDir { name, path });
            // A skill dir is a leaf for discovery — do not descend into it.
            continue;
        }
        collect(root, &path, out);
    }
}
