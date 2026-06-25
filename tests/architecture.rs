//! Enforces the core/shell seam: nothing under src/core/ may depend on a UI
//! crate. This is the compiler-substitute that keeps `core` pure while the
//! project ships as a single crate.

use std::fs;
use std::path::Path;

const FORBIDDEN: &[&str] = &["ratatui", "crossterm", "clap"];

#[test]
fn core_has_no_ui_dependencies() {
    let core_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core");
    let mut violations = Vec::new();
    visit(&core_dir, &mut |path| {
        let content = fs::read_to_string(path).expect("read core file");
        for (i, line) in content.lines().enumerate() {
            for &name in FORBIDDEN {
                if line.contains(&format!("use {name}")) || line.contains(&format!("{name}::")) {
                    violations.push(format!(
                        "{}:{}: forbidden UI crate `{}` in core: {}",
                        path.display(),
                        i + 1,
                        name,
                        line.trim()
                    ));
                }
            }
        }
    });
    assert!(
        violations.is_empty(),
        "src/core must not depend on UI crates:\n{}",
        violations.join("\n")
    );
}

fn visit(dir: &Path, f: &mut dyn FnMut(&Path)) {
    for entry in fs::read_dir(dir).expect("read core dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            visit(&path, f);
        } else if path.extension().is_some_and(|e| e == "rs") {
            f(&path);
        }
    }
}
