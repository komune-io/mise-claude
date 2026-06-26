use chord::core::skills::discover::{discover_all, find_one};
use std::fs;
use std::path::Path;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn collection_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("skills/foo/SKILL.md"), "# foo\n");
    write(&r.join("skills/bar/SKILL.md"), "# bar\n");
    write(&r.join("template/SKILL.md"), "# template\n");
    write(&r.join("README.md"), "x");
    dir
}

#[test]
fn discover_all_finds_collection_skills_sorted_excluding_template() {
    let dir = collection_repo();
    let skills = discover_all(dir.path(), "repo");
    let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["bar", "foo"]);
}

#[test]
fn discover_all_treats_root_skill_md_as_single_skill() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("SKILL.md"), "# solo\n");
    let skills = discover_all(dir.path(), "solo");
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "solo");
}

#[test]
fn find_one_returns_named_skill() {
    let dir = collection_repo();
    let s = find_one(dir.path(), "repo", "foo").unwrap();
    assert_eq!(s.name, "foo");
    assert!(s.path.join("SKILL.md").exists());
}

#[test]
fn find_one_none_for_missing() {
    let dir = collection_repo();
    assert!(find_one(dir.path(), "repo", "template").is_none());
    assert!(find_one(dir.path(), "repo", "nope").is_none());
}
