mod config;
mod task;
mod vault;
mod view;
mod ui;
mod storage;

use config::Config;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = Config::load().unwrap_or_else(|_| {
        // Default config for development
        Config {
            workspace: config::WorkspaceConfig {
                path: ".".to_string(),
            },
            defaults: config::DefaultsConfig::default(),
            theme: config::ThemeConfig::default(),
        }
    });

    let workspace_path = PathBuf::from(&config.workspace.path);
    let md_files = vault::find_markdown_files(&workspace_path);

    let mut all_tasks = Vec::new();
    for file in &md_files {
        if let Ok(content) = std::fs::read_to_string(file) {
            let tasks = task::parser::parse_file(&content, file);
            all_tasks.extend(tasks);
        }
    }

    let views_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("taskboard")
        .join("views.toml");
    let views = storage::load_views(&views_path)?;

    // Set up filesystem watcher
    let file_watcher = vault::FileWatcher::new(&workspace_path).ok();

    let mut terminal = ratatui::init();
    let mut app = ui::App::new(config, all_tasks, views);
    app.file_watcher = file_watcher;
    app.run(&mut terminal)?;
    ratatui::restore();

    Ok(())
}
