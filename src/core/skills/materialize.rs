//! Materialize a discovered skill into the chord-owned `.chord/` store and
//! expose it to Claude Code via a relative symlink under `.claude/skills/`.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::discover::SkillDir;
use super::SkillError;

/// `<project_root>/.chord/<owner>/<repo>/<name>`.
pub fn store_path(project_root: &Path, owner_repo: &str, skill_name: &str) -> PathBuf {
    let mut p = project_root.join(".chord");
    for seg in owner_repo.split('/') {
        p = p.join(seg);
    }
    p.join(skill_name)
}

/// Copy the skill into the store, create the relative symlink, and return
/// the store's integrity hash.
pub fn materialize(
    project_root: &Path,
    owner_repo: &str,
    skill: &SkillDir,
) -> Result<String, SkillError> {
    let store = store_path(project_root, owner_repo, &skill.name);

    // Fresh store: remove any stale copy, then recursively copy.
    if store.exists() {
        std::fs::remove_dir_all(&store)
            .map_err(|e| SkillError::Io(store.display().to_string(), e.to_string()))?;
    }
    copy_dir(&skill.path, &store)?;

    create_symlink(project_root, owner_repo, &skill.name)?;

    integrity(&store)
}

/// sha256 over a path-sorted walk of `dir`: for each file, hash its
/// relative path then its bytes. Returns `sha256:<lowercase-hex>`.
pub fn integrity(dir: &Path) -> Result<String, SkillError> {
    let mut files = Vec::new();
    walk_files(dir, dir, &mut files)?;
    files.sort();

    let mut hasher = Sha256::new();
    for rel in &files {
        hasher.update(rel.as_bytes());
        hasher.update([0u8]);
        let bytes =
            std::fs::read(dir.join(rel)).map_err(|e| SkillError::Io(rel.clone(), e.to_string()))?;
        hasher.update(&bytes);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn create_symlink(
    project_root: &Path,
    owner_repo: &str,
    skill_name: &str,
) -> Result<(), SkillError> {
    let link_dir = project_root.join(".claude").join("skills");
    let link = link_dir.join(skill_name);
    // depth of .claude/skills/<name> below project root is 2 dirs up to root.
    let mut rel = PathBuf::from("../../.chord");
    for seg in owner_repo.split('/') {
        rel = rel.join(seg);
    }
    let rel = rel.join(skill_name);

    // Collision check: an existing symlink to a different target, or a real
    // directory not owned by us, is a hard error.
    if let Ok(meta) = std::fs::symlink_metadata(&link) {
        if meta.file_type().is_symlink() {
            let current = std::fs::read_link(&link).unwrap_or_default();
            if current == rel {
                return Ok(()); // already correct
            }
            return Err(SkillError::NameCollision(
                skill_name.to_string(),
                current.display().to_string(),
                owner_repo.to_string(),
            ));
        }
        return Err(SkillError::NameCollision(
            skill_name.to_string(),
            link.display().to_string(),
            owner_repo.to_string(),
        ));
    }

    std::fs::create_dir_all(&link_dir)
        .map_err(|e| SkillError::Io(link_dir.display().to_string(), e.to_string()))?;
    std::os::unix::fs::symlink(&rel, &link)
        .map_err(|e| SkillError::Io(link.display().to_string(), e.to_string()))
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), SkillError> {
    std::fs::create_dir_all(dst)
        .map_err(|e| SkillError::Io(dst.display().to_string(), e.to_string()))?;
    let entries = std::fs::read_dir(src)
        .map_err(|e| SkillError::Io(src.display().to_string(), e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| SkillError::Io(src.display().to_string(), e.to_string()))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| SkillError::Io(to.display().to_string(), e.to_string()))?;
        }
    }
    Ok(())
}

fn walk_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), SkillError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| SkillError::Io(dir.display().to_string(), e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| SkillError::Io(dir.display().to_string(), e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            walk_files(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| SkillError::Io(path.display().to_string(), e.to_string()))?
                .to_string_lossy()
                .into_owned();
            out.push(rel);
        }
    }
    Ok(())
}
