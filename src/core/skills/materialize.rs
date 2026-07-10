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

/// The flat directory name a skill is exposed under in `.claude/skills/`.
///
/// Claude Code only discovers skills one level deep (`.claude/skills/*/SKILL.md`),
/// so the source namespace is folded into the single link name with `__`:
/// `"mattpocock/skills"` + `"ask-matt"` → `"mattpocock__skills__ask-matt"`.
/// This keeps discovery working while making provenance visible and preventing
/// same-named skills from different repos from colliding.
pub fn link_name(owner_repo: &str, skill_name: &str) -> String {
    format!("{}__{}", owner_repo.replace('/', "__"), skill_name)
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
    let link = link_dir.join(link_name(owner_repo, skill_name));
    // depth of .claude/skills/<name> below project root is 2 dirs up to root.
    let mut rel = PathBuf::from("../../.chord");
    for seg in owner_repo.split('/') {
        rel = rel.join(seg);
    }
    let rel = rel.join(skill_name);

    // Migration: drop a legacy pre-namespacing bare-name link (`.claude/skills/
    // <skill_name>`) if it is a chord-owned symlink, so upgrading from the flat
    // scheme does not leave an orphan alongside the new namespaced link. The
    // store was copied above, so the target resolves for the ownership check.
    remove_legacy_bare_link(project_root, &link_dir.join(skill_name));

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

/// Best-effort removal of a legacy bare-name link at `path` iff it is a symlink
/// whose target resolves inside `<project_root>/.chord`. Foreign symlinks and
/// real directories are left untouched.
fn remove_legacy_bare_link(project_root: &Path, path: &Path) {
    if symlink_points_into_chord(project_root, path) {
        let _ = std::fs::remove_file(path);
    }
}

/// True iff `link` is a symlink whose target resolves inside `<root>/.chord`.
/// Canonicalizes both sides against the real filesystem, so a foreign symlink
/// that merely traverses an unrelated `.chord` segment is not mistaken for
/// chord-owned. Requires the target to exist (dangling links read as foreign).
pub(crate) fn symlink_points_into_chord(root: &Path, link: &Path) -> bool {
    let Ok(target) = std::fs::read_link(link) else {
        return false;
    };
    let resolved = link.parent().unwrap_or(Path::new(".")).join(target);
    match (
        std::fs::canonicalize(&resolved),
        std::fs::canonicalize(root.join(".chord")),
    ) {
        (Ok(r), Ok(chord)) => r.starts_with(chord),
        _ => false,
    }
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
