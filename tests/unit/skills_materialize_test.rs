use chord::core::skills::discover::SkillDir;
use chord::core::skills::materialize::{integrity, materialize, store_path};
use std::fs;
use std::path::Path;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn src_skill(name: &str) -> (tempfile::TempDir, SkillDir) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join(name);
    write(&p.join("SKILL.md"), "---\nname: x\n---\nbody\n");
    write(&p.join("scripts/run.py"), "print(1)\n");
    (
        dir,
        SkillDir {
            name: name.to_string(),
            path: p,
        },
    )
}

#[test]
fn store_path_is_chord_owner_repo_name() {
    let pr = Path::new("/proj");
    assert_eq!(
        store_path(pr, "anthropics/skills", "pdf"),
        Path::new("/proj/.chord/anthropics/skills/pdf")
    );
}

#[test]
fn integrity_is_deterministic_and_content_sensitive() {
    let (src, skill) = src_skill("foo");
    let a = integrity(&skill.path).unwrap();
    let b = integrity(&skill.path).unwrap();
    assert_eq!(a, b);
    assert!(a.starts_with("sha256:"));
    write(&skill.path.join("SKILL.md"), "---\nname: x\n---\nCHANGED\n");
    let c = integrity(&skill.path).unwrap();
    assert_ne!(a, c);
    drop(src);
}

#[test]
fn materialize_copies_store_and_makes_relative_symlink() {
    let proj = tempfile::tempdir().unwrap();
    let (src, skill) = src_skill("pdf");
    let hash = materialize(proj.path(), "anthropics/skills", &skill).unwrap();
    assert!(hash.starts_with("sha256:"));

    let store = proj.path().join(".chord/anthropics/skills/pdf");
    assert!(store.join("SKILL.md").is_file());
    assert!(store.join("scripts/run.py").is_file());

    let link = proj.path().join(".claude/skills/anthropics__skills__pdf");
    let target = fs::read_link(&link).unwrap();
    assert_eq!(target, Path::new("../../.chord/anthropics/skills/pdf"));
    // The symlink resolves to the store.
    assert!(link.join("SKILL.md").is_file());
    drop(src);
}

#[test]
fn materialize_is_idempotent_for_same_target() {
    let proj = tempfile::tempdir().unwrap();
    let (src, skill) = src_skill("pdf");
    let h1 = materialize(proj.path(), "anthropics/skills", &skill).unwrap();
    let h2 = materialize(proj.path(), "anthropics/skills", &skill).unwrap();
    assert_eq!(h1, h2);
    let link = proj.path().join(".claude/skills/anthropics__skills__pdf");
    assert_eq!(
        std::fs::read_link(&link).unwrap(),
        std::path::Path::new("../../.chord/anthropics/skills/pdf")
    );
    assert!(link.join("SKILL.md").is_file());
    drop(src);
}

#[test]
fn materialize_errors_on_foreign_symlink_collision() {
    let proj = tempfile::tempdir().unwrap();
    // Pre-create the namespaced link pointing somewhere else.
    let link = proj.path().join(".claude/skills/anthropics__skills__pdf");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink("../../somewhere/else", &link).unwrap();

    let (src, skill) = src_skill("pdf");
    let err = materialize(proj.path(), "anthropics/skills", &skill);
    assert!(matches!(
        err,
        Err(chord::core::skills::SkillError::NameCollision(..))
    ));
    drop(src);
}

#[test]
fn link_name_folds_owner_repo_into_flat_name() {
    use chord::core::skills::materialize::link_name;
    assert_eq!(
        link_name("mattpocock/skills", "ask-matt"),
        "mattpocock__skills__ask-matt"
    );
}

#[test]
fn materialize_removes_legacy_bare_link() {
    let proj = tempfile::tempdir().unwrap();
    // Simulate a pre-namespacing install: bare-name chord-owned symlink.
    let bare = proj.path().join(".claude/skills/pdf");
    fs::create_dir_all(bare.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink("../../.chord/anthropics/skills/pdf", &bare).unwrap();

    let (src, skill) = src_skill("pdf");
    materialize(proj.path(), "anthropics/skills", &skill).unwrap();

    // Legacy bare link is gone; namespaced link took its place.
    assert!(bare.symlink_metadata().is_err(), "legacy bare link removed");
    assert!(proj
        .path()
        .join(".claude/skills/anthropics__skills__pdf")
        .join("SKILL.md")
        .is_file());
    drop(src);
}

#[test]
fn materialize_keeps_foreign_bare_link() {
    let proj = tempfile::tempdir().unwrap();
    // A same-named foreign symlink (not into .chord) must be left alone.
    let bare = proj.path().join(".claude/skills/pdf");
    fs::create_dir_all(bare.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink("../../.agents/skills/pdf", &bare).unwrap();

    let (src, skill) = src_skill("pdf");
    materialize(proj.path(), "anthropics/skills", &skill).unwrap();

    assert!(bare.symlink_metadata().is_ok(), "foreign bare link kept");
    drop(src);
}
