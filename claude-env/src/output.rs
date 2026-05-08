pub struct Reporter {
    pub installed: u32,
    pub skipped: u32,
    pub failed: u32,
    quiet: bool,
}

impl Reporter {
    pub fn new() -> Self {
        Self { installed: 0, skipped: 0, failed: 0, quiet: false }
    }

    pub fn new_quiet() -> Self {
        Self { installed: 0, skipped: 0, failed: 0, quiet: true }
    }

    pub fn success(&mut self, name: &str, version: &str, detail: &str) {
        self.installed += 1;
        // Print even in quiet mode: something changed and the user should see it.
        println!("  \x1b[32m✓\x1b[0m {:<25} {} {}", name, version, detail);
    }

    pub fn failure(&mut self, name: &str, version: &str, error: &str) {
        self.failed += 1;
        eprintln!("  \x1b[31m✗\x1b[0m {:<25} {} failed", name, version);
        for line in error.lines() {
            eprintln!("    \x1b[90m│\x1b[0m {}", line);
        }
    }

    pub fn skip(&mut self, name: &str, version: &str) {
        self.skipped += 1;
        if !self.quiet {
            println!("  \x1b[90m⊘\x1b[0m {:<25} {} skipped", name, version);
        }
    }

    pub fn summary(&self) {
        if self.quiet && self.installed == 0 && self.failed == 0 {
            return;
        }
        println!();
        println!(
            "  {} installed, {} failed, {} skipped",
            self.installed, self.failed, self.skipped
        );
    }

    pub fn exit_code(&self) -> i32 {
        if self.failed > 0 { 1 } else { 0 }
    }
}

impl Default for Reporter {
    fn default() -> Self {
        Self::new()
    }
}
