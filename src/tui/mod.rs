pub mod actions;
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
use crate::inspect::{reconciler, scanner, AuditReport, Category};
use app::App;
use tree::build_tree;

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
    let mut app = App::new(tree_nodes);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app, home_dir);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    home_dir: &Path,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handler::handle_key(app, key, home_dir)?;
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
