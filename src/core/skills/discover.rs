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
/// - A root `SKILL.md` means the whole repo is one skill (named after the
///   repo directory). Returned as a single entry.
/// - Otherwise, every directory containing a `SKILL.md` is a skill (the
///   whole directory is its payload). A top-level `template/` directory is
///   excluded — it is scaffolding, not an installable skill.
///
/// Result is sorted by name for determinism.
pub fn discover_all(root: &Path) -> Vec<SkillDir> {
    if root.join("SKILL.md").is_file() {
        let skill_md_path = root.join("SKILL.md");
        let name = extract_name_from_skill_md(&skill_md_path)
            .unwrap_or_else(|| {
                root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "skill".to_string())
            });
        return vec![SkillDir {
            name,
            path: root.to_path_buf(),
        }];
    }

    let mut found = Vec::new();
    collect(root, root, &mut found);
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Find a single skill by name (directory basename). Returns None if no
/// skill directory with that basename exists.
pub fn find_one(root: &Path, name: &str) -> Option<SkillDir> {
    discover_all(root).into_iter().find(|s| s.name == name)
}

fn extract_name_from_skill_md(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("name:") {
            return Some(rest.trim().to_string());
        }
    }
    None
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
            let skill_md_path = path.join("SKILL.md");
            let name = extract_name_from_skill_md(&skill_md_path)
                .unwrap_or_else(|| {
                    path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                });
            out.push(SkillDir { name, path });
            // A skill dir is a leaf for discovery — do not descend into it.
            continue;
        }
        collect(root, &path, out);
    }
}
