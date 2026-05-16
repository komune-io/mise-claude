pub mod app;
pub mod handler;
pub(crate) mod markdown;
pub mod tree;
pub mod ui;

use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;

use crate::config::Config;
use crate::inspect::{reconciler, scanner, AuditReport, Category, Scope};
use crate::installer::DefaultInstallers;
use crate::operations::add::AddSpec;
use crate::operations::OperationError;
use crate::store::{FileConfigStore, FileLockfileStore};
use app::App;
use tree::build_tree;

/// Indirection for operations called from the TUI handler.
///
/// The production impl forwards to `crate::operations::*`. Tests use a
/// recording mock to verify which calls the handler made without running
/// real subprocesses.
pub trait OpRunner {
    fn add(&mut self, spec: &AddSpec) -> Result<(), OperationError>;
    fn remove(&mut self, name: &str) -> Result<(), OperationError>;
    fn install_one(&mut self, name: &str) -> Result<(), OperationError>;
    fn install_all(&mut self) -> Result<(), OperationError>;
    fn set_scope(
        &mut self,
        plugin_id: &str,
        scope: Scope,
        enabled: bool,
    ) -> Result<(), OperationError>;
}

/// Production [`OpRunner`] that calls `crate::operations::*`.
///
/// Owns the File* store adapters and the default installer set for the
/// session so each `ctx()` call borrows the same instances. The trait
/// methods are `&self`; the file adapters and Installer impls don't
/// internally mutate, so this stays cheap.
pub struct DefaultOpRunner<'a> {
    pub project_root: &'a std::path::Path,
    pub home_dir: &'a std::path::Path,
    pub packages_dir: &'a std::path::Path,
    pub verbose: bool,
    config_store: FileConfigStore,
    lockfile_store: FileLockfileStore,
    installers: DefaultInstallers,
}

impl<'a> DefaultOpRunner<'a> {
    pub fn new(
        project_root: &'a std::path::Path,
        home_dir: &'a std::path::Path,
        packages_dir: &'a std::path::Path,
        verbose: bool,
    ) -> Self {
        let config_store = FileConfigStore::new(project_root);
        let lockfile_store = FileLockfileStore::new(project_root);
        let installers = DefaultInstallers::new();
        Self {
            project_root,
            home_dir,
            packages_dir,
            verbose,
            config_store,
            lockfile_store,
            installers,
        }
    }

    /// Build an `OpContext` for a single operation. The returned context
    /// borrows from `self` plus the local `installer_set` the caller
    /// provides. `OpRunner` methods below pass a stack-allocated set
    /// derived from `self.installers` — Rust can't return a struct that
    /// borrows from a local, so each method builds the set inline.
    fn ctx<'b>(
        &'b self,
        installer_set: &'b crate::installer::InstallerSet<'b>,
    ) -> crate::operations::OpContext<'b> {
        crate::operations::OpContext {
            config_store: &self.config_store,
            lockfile_store: &self.lockfile_store,
            installers: installer_set,
            project_root: self.project_root,
            home_dir: self.home_dir,
            packages_dir: self.packages_dir,
            verbose: self.verbose,
        }
    }
}

impl<'a> OpRunner for DefaultOpRunner<'a> {
    fn add(&mut self, spec: &AddSpec) -> Result<(), OperationError> {
        let set = self.installers.as_set();
        crate::operations::add::write_toml_entry(spec, &self.ctx(&set))
    }
    fn remove(&mut self, name: &str) -> Result<(), OperationError> {
        let set = self.installers.as_set();
        crate::operations::remove::remove(name, &self.ctx(&set)).map(|_| ())
    }
    fn install_one(&mut self, name: &str) -> Result<(), OperationError> {
        let set = self.installers.as_set();
        crate::operations::install::install_one(name, &self.ctx(&set), false).map(|_| ())
    }
    fn install_all(&mut self) -> Result<(), OperationError> {
        let set = self.installers.as_set();
        crate::operations::install::install_all(&self.ctx(&set), false).map(|_| ())
    }
    fn set_scope(
        &mut self,
        plugin_id: &str,
        scope: Scope,
        enabled: bool,
    ) -> Result<(), OperationError> {
        let set = self.installers.as_set();
        crate::operations::scope::set_plugin_enabled(plugin_id, scope, enabled, &self.ctx(&set))
    }
}

pub fn run_tui(project_root: &Path, home_dir: &Path, config: &Config) -> io::Result<()> {
    let enabled_plugins = scanner::collect_enabled_plugins(project_root, home_dir);
    let mut report_entries = Vec::new();
    for category in Category::all() {
        let discovered = match category {
            Category::Mcp => scanner::scan_mcp(project_root, home_dir),
            Category::Plugins => scanner::scan_plugins(project_root, home_dir),
            Category::Skills => scanner::scan_skills(project_root, home_dir),
            Category::Commands => scanner::scan_commands(project_root, home_dir),
            Category::Agents => scanner::scan_agents(project_root, home_dir),
            Category::Hooks => scanner::scan_hooks(project_root, home_dir),
        };
        let entries =
            reconciler::reconcile(category.clone(), &discovered, config, &enabled_plugins);
        report_entries.push((category, entries));
    }
    let report = AuditReport {
        entries: report_entries,
    };
    let tree_nodes = build_tree(&report);
    let mut app = App::new(tree_nodes, Some(home_dir.to_path_buf()));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let packages_dir = crate::operations::install::default_packages_dir();
    let result = run_loop(
        &mut terminal,
        &mut app,
        project_root,
        home_dir,
        &packages_dir,
        config,
    );

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    project_root: &Path,
    home_dir: &Path,
    packages_dir: &Path,
    _config: &Config,
) -> io::Result<()> {
    let mut runner = DefaultOpRunner::new(project_root, home_dir, packages_dir, false);

    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handler::handle_key_with_runner(app, key, &mut runner)?;

                // Drain any queued inline operation. These drop out of the alt-screen
                // to show subprocess output, run the op via the runner, then mark
                // dirty so the tree refreshes below.
                if let Some(op) = app.pending_inline_op.take() {
                    use crate::tui::app::InlineOp;
                    let result = match &op {
                        InlineOp::InstallOne(name) => {
                            let header = format!("chord install {name}");
                            run_inline(terminal, &header, || runner.install_one(name))?
                        }
                        InlineOp::InstallAll => {
                            let header = "chord install".to_string();
                            run_inline(terminal, &header, || runner.install_all())?
                        }
                    };
                    match result {
                        Ok(()) => app.set_status("Install complete".to_string()),
                        Err(e) => app.set_status(format!("Install failed: {e}")),
                    }
                    app.dirty = true;
                }

                if app.dirty {
                    let cfg_path = project_root.join("chord.toml");
                    let fresh_config =
                        crate::config::Config::from_file(&cfg_path).unwrap_or_default();
                    app.reload(project_root, home_dir, &fresh_config);
                    app.dirty = false;
                }
            }
        }
        if let Some((_, time)) = &app.status_message {
            if time.elapsed() > Duration::from_secs(3) {
                app.status_message = None;
            }
        }
        if app.should_quit {
            break;
        }
    }
    Ok(())
}

/// Temporarily leave the alternate screen + raw mode, run `f`, then
/// re-enter. Used for slow subprocess operations (`npm`, `claude`, `npx`)
/// so the user sees the install stream live.
///
/// The closure receives no arguments; it should not touch the terminal
/// itself. After the closure returns, prints "press any key to return"
/// and blocks on a single event before re-entering.
pub fn run_inline<F, T>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    header: &str,
    f: F,
) -> io::Result<T>
where
    F: FnOnce() -> T,
{
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    println!("▶ {header}");
    let result = f();
    println!("\n[Press any key to return]");
    // Drain any pending events, then wait for one.
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }
    let _ = event::read()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Ok(result)
}
