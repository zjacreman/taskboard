mod config;
mod task;
mod vault;
mod view;
mod ui;

#[cfg(test)]
mod test_helpers;

use config::Config;
use std::path::PathBuf;

fn prompt_workspace_path() -> PathBuf {
    let mut rl = rustyline::DefaultEditor::new().expect("Failed to create editor");

    let input = rl.readline("Enter Obsidian vault path: ")
        .expect("Failed to read input");

    let trimmed = input.trim();

    // Strip surrounding quotes if present
    let unquoted = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    // Expand ~ to home directory
    let expanded = if let Some(stripped) = unquoted.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            home.join(stripped.trim_start_matches('/'))
        } else {
            PathBuf::from(unquoted)
        }
    } else {
        PathBuf::from(unquoted)
    };

    expanded
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = match Config::load() {
        Ok(config) => config,
        Err(_) => {
            let workspace_path = prompt_workspace_path();
            if !workspace_path.exists() {
                eprintln!("Error: path does not exist: {}", workspace_path.display());
                std::process::exit(1);
            }

            let config = Config {
                workspace: config::WorkspaceConfig {
                    path: workspace_path,
                },
                defaults: config::DefaultsConfig::default(),
                theme: config::ThemeConfig::default(),
                views: vec![],
            };

            // Save config for next run
            if let Some(config_dir) = dirs::config_dir() {
                let config_path = config_dir.join("taskboard").join("config.toml");
                if let Err(e) = std::fs::create_dir_all(config_path.parent().unwrap()) {
                    eprintln!("Warning: could not create config directory: {}", e);
                } else {
                    let toml = format!(
                        "[workspace]\npath = \"{}\"\n\n[defaults]\nview = \"All Tasks\"\n\n[theme]\ncolors = \"dark\"\n\n[[views]]\nname = \"All Tasks\"\nquery = \"\"\nsort_by = \"due_date\"\ngroup_by = \"\"\n",
                        config.workspace.path.display()
                    );
                    if let Err(e) = std::fs::write(&config_path, toml) {
                        eprintln!("Warning: could not write config: {}", e);
                    } else {
                        eprintln!("Config saved to {}", config_path.display());
                    }
                }
            }

            config
        }
    };

    let workspace_path = config.workspace.path.clone();
    let md_files = vault::find_markdown_files(&workspace_path);

    let mut all_tasks = Vec::new();
    for file in &md_files {
        if let Ok(content) = std::fs::read_to_string(file) {
            let tasks = task::parser::parse_file(&content, file);
            all_tasks.extend(tasks);
        }
    }

    let config_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("taskboard")
        .join("config.toml");

    let views: Vec<crate::view::View> = config.views.iter().map(|v| {
        crate::view::View::new(&v.name, &v.query, &v.sort_by, &v.group_by)
    }).collect();

    let file_watcher = vault::FileWatcher::new(&workspace_path).ok();

    let mut terminal = ratatui::init();
    let _guard = TerminalGuard;
    let mut app = ui::App::new(config, all_tasks, views, config_path);
    app.file_watcher = file_watcher;
    app.run(&mut terminal)?;

    Ok(())
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}
