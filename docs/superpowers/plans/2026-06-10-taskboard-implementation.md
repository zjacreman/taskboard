# TaskBoard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a TUI app in Rust that manages Obsidian-style markdown tasks with full query, filter, and edit capabilities.

**Architecture:** Single-pane Ratatui TUI with modular backend — parser, query engine, vault walker, and filesystem watcher are separate from UI. TDD throughout.

**Tech Stack:** Rust, Ratatui, crossterm, notify, rayon, chrono, serde, toml

---

## File Structure

```
taskboard/
├── Cargo.toml
├── shell.nix
├── src/
│   ├── main.rs              — entry point, CLI args, app init
│   ├── config.rs            — config.toml loading/saving
│   ├── task/
│   │   ├── mod.rs           — Task struct, TaskStatus, Priority types
│   │   ├── parser.rs        — extract tasks from markdown lines
│   │   └── query.rs         — query engine (tokenize, filter, sort, group)
│   ├── vault.rs             — filesystem walker, notify watcher
│   ├── view.rs              — View struct, display settings
│   ├── ui/
│   │   ├── mod.rs           — App state, event loop, input handling
│   │   ├── task_list.rs     — main task list widget
│   │   ├── modal.rs         — task editor modal
│   │   ├── command.rs       — inline search/query input
│   │   └── theme.rs         — color schemes, responsive breakpoints
│   └── storage.rs           — views.toml read/write
├── tests/
│   ├── fixtures/
│   │   ├── basic.md         — simple tasks
│   │   ├── full_metadata.md — tasks with all emoji fields
│   │   ├── edge_cases.md    — malformed, nested, multi-line
│   │   └── large.md         — generated file with 1000 tasks
│   ├── parser_test.rs
│   ├── query_test.rs
│   ├── config_test.rs
│   ├── vault_test.rs
│   ├── view_test.rs
│   └── integration_test.rs
```

---

## Task 1: Project Setup

**Files:**
- Create: `Cargo.toml`
- Create: `shell.nix`
- Create: `src/main.rs`

- [ ] **Step 1: Update shell.nix with all required packages**

```nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    clippy
    rustfmt
    pkg-config
    openssl
  ];
}
```

- [ ] **Step 2: Create Cargo.toml**

```toml
[package]
name = "taskboard"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
crossterm = "0.28"
notify = "6"
rayon = "1"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
dirs = "5"
log = "0.4"
env_logger = "0.10"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Create minimal main.rs**

```rust
fn main() {
    println!("TaskBoard starting...");
}
```

- [ ] **Step 4: Verify it compiles**

Run: `nix-shell --run "cargo build"`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml shell.nix src/main.rs
git commit -m "feat: initial project setup with dependencies"
```

---

## Task 2: Task Model

**Files:**
- Create: `src/task/mod.rs`

- [ ] **Step 1: Write tests for Task types**

```rust
// src/task/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_cycle() {
        let mut status = TaskStatus::Todo;
        status.cycle();
        assert_eq!(status, TaskStatus::Done);
        status.cycle();
        assert_eq!(status, TaskStatus::Todo);
    }

    #[test]
    fn test_priority_cycle() {
        let mut priority = Priority::None;
        priority.cycle();
        assert_eq!(priority, Priority::High);
        priority.cycle();
        assert_eq!(priority, Priority::Medium);
        priority.cycle();
        assert_eq!(priority, Priority::Low);
        priority.cycle();
        assert_eq!(priority, Priority::Lowest);
        priority.cycle();
        assert_eq!(priority, Priority::None);
    }

    #[test]
    fn test_priority_emoji_roundtrip() {
        assert_eq!(Priority::from_emoji("⏫"), Some(Priority::High));
        assert_eq!(Priority::from_emoji("🔼"), Some(Priority::Medium));
        assert_eq!(Priority::from_emoji("🔽"), Some(Priority::Low));
        assert_eq!(Priority::from_emoji("⏬"), Some(Priority::Lowest));
        assert_eq!(Priority::from_emoji("x"), None);

        assert_eq!(Priority::High.to_emoji(), "⏫");
        assert_eq!(Priority::None.to_emoji(), "");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix-shell --run "cargo test --lib"`
Expected: FAIL with "TaskStatus not found"

- [ ] **Step 3: Implement Task types**

```rust
// src/task/mod.rs
use chrono::NaiveDate;
use std::path::PathBuf;

pub mod parser;
pub mod query;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Todo,
    Done,
}

impl TaskStatus {
    pub fn cycle(&mut self) {
        *self = match self {
            TaskStatus::Todo => TaskStatus::Done,
            TaskStatus::Done => TaskStatus::Todo,
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Lowest,
    Low,
    Medium,
    High,
    None,
}

impl Priority {
    pub fn cycle(&mut self) {
        *self = match self {
            Priority::None => Priority::High,
            Priority::High => Priority::Medium,
            Priority::Medium => Priority::Low,
            Priority::Low => Priority::Lowest,
            Priority::Lowest => Priority::None,
        };
    }

    pub fn from_emoji(s: &str) -> Option<Priority> {
        match s {
            "⏫" => Some(Priority::High),
            "🔼" => Some(Priority::Medium),
            "🔽" => Some(Priority::Low),
            "⏬" => Some(Priority::Lowest),
            _ => None,
        }
    }

    pub fn to_emoji(&self) -> &str {
        match self {
            Priority::High => "⏫",
            Priority::Medium => "🔼",
            Priority::Low => "🔽",
            Priority::Lowest => "⏬",
            Priority::None => "",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub scheduled_date: Option<NaiveDate>,
    pub recurrence: Option<String>,
    pub done_date: Option<NaiveDate>,
    pub start_date: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub source_file: PathBuf,
    pub line_number: usize,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix-shell --run "cargo test --lib"`
Expected: All 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/task/mod.rs
git commit -m "feat: add Task, TaskStatus, and Priority types"
```

---

## Task 3: Task Parser

**Files:**
- Create: `src/task/parser.rs`
- Create: `tests/fixtures/basic.md`
- Create: `tests/fixtures/full_metadata.md`
- Create: `tests/fixtures/edge_cases.md`

- [ ] **Step 1: Create test fixtures**

```markdown
<!-- tests/fixtures/basic.md -->
# My Tasks

- [ ] Buy groceries
- [x] Review pull request
- [ ] Write documentation

## Project Tasks

- [ ] Fix authentication bug
- [x] Deploy to staging
```

```markdown
<!-- tests/fixtures/full_metadata.md -->
# Full Metadata Tasks

- [ ] Task with due date 📅 2026-06-15
- [ ] Task with scheduled date ⏳ 2026-06-12
- [ ] Task with recurrence 🔁 every week
- [ ] Task with priority ⏫
- [ ] Task with all fields 📅 2026-06-15 ⏳ 2026-06-12 🔁 every week ⏫
- [x] Done task with done date ✅ 2026-06-10
- [ ] Task with tags #work #urgent
```

```markdown
<!-- tests/fixtures/edge_cases.md -->
# Edge Cases

- [ ]   Task with extra spaces
- [ ]Task with no space after bracket
- [x] Done task with no done date
- [ ] Task with emoji in description 🎉
- [ ] Task with special chars: <>&"'
- [ ] Multi-line
  description continues here
- [ ] Nested task
  - [ ] Sub-task
  - [x] Done sub-task
- Not a task (no checkbox)
- [ ] Task with date in different format: 2026/06/15
```

- [ ] **Step 2: Write parser tests**

```rust
// src/task/parser.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_basic_tasks() {
        let content = std::fs::read_to_string("tests/fixtures/basic.md").unwrap();
        let tasks = parse_file(&content, &PathBuf::from("tests/fixtures/basic.md"));

        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks[0].description, "Buy groceries");
        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert_eq!(tasks[1].description, "Review pull request");
        assert_eq!(tasks[1].status, TaskStatus::Done);
    }

    #[test]
    fn test_parse_full_metadata() {
        let content = std::fs::read_to_string("tests/fixtures/full_metadata.md").unwrap();
        let tasks = parse_file(&content, &PathBuf::from("tests/fixtures/full_metadata.md"));

        let due_task = &tasks[0];
        assert_eq!(due_task.description, "Task with due date");
        assert_eq!(due_task.due_date, Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()));

        let scheduled_task = &tasks[1];
        assert_eq!(scheduled_task.scheduled_date, Some(NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()));

        let recurrence_task = &tasks[2];
        assert_eq!(recurrence_task.recurrence, Some("every week".to_string()));

        let priority_task = &tasks[3];
        assert_eq!(priority_task.priority, Priority::High);

        let all_fields = &tasks[4];
        assert_eq!(all_fields.due_date, Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()));
        assert_eq!(all_fields.scheduled_date, Some(NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()));
        assert_eq!(all_fields.recurrence, Some("every week".to_string()));
        assert_eq!(all_fields.priority, Priority::High);

        let done_task = &tasks[5];
        assert_eq!(done_task.done_date, Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()));

        let tagged_task = &tasks[6];
        assert_eq!(tagged_task.tags, vec!["work", "urgent"]);
    }

    #[test]
    fn test_parse_edge_cases() {
        let content = std::fs::read_to_string("tests/fixtures/edge_cases.md").unwrap();
        let tasks = parse_file(&content, &PathBuf::from("tests/fixtures/edge_cases.md"));

        // Should parse tasks with extra spaces
        assert!(tasks.iter().any(|t| t.description == "Task with extra spaces"));

        // Should handle special characters
        assert!(tasks.iter().any(|t| t.description.contains("<>&\"'")));

        // Should find nested sub-tasks
        assert!(tasks.iter().any(|t| t.description == "Sub-task"));
        assert!(tasks.iter().any(|t| t.description == "Done sub-task"));

        // Should not include non-task lines
        assert!(!tasks.iter().any(|t| t.description == "Not a task (no checkbox)"));
    }

    #[test]
    fn test_line_numbers() {
        let content = "- [ ] First\n- [ ] Second\n- [x] Third";
        let tasks = parse_file(content, &PathBuf::from("test.md"));

        assert_eq!(tasks[0].line_number, 1);
        assert_eq!(tasks[1].line_number, 2);
        assert_eq!(tasks[2].line_number, 3);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `nix-shell --run "cargo test --lib"`
Expected: FAIL with "parse_file not found"

- [ ] **Step 4: Implement parser**

```rust
// src/task/parser.rs
use super::{Task, TaskStatus, Priority};
use chrono::NaiveDate;
use std::path::PathBuf;

pub fn parse_file(content: &str, source_file: &PathBuf) -> Vec<Task> {
    let mut tasks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        if let Some(task) = parse_line(line, line_num + 1, source_file) {
            tasks.push(task);
        }
    }

    tasks
}

fn parse_line(line: &str, line_number: usize, source_file: &PathBuf) -> Option<Task> {
    let trimmed = line.trim();

    // Must start with "- [ ]" or "- [x]"
    let (status, rest) = if trimmed.starts_with("- [ ]") {
        (TaskStatus::Todo, &trimmed[5..])
    } else if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
        (TaskStatus::Done, &trimmed[5..])
    } else {
        return None;
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    let mut description = String::new();
    let mut due_date = None;
    let mut scheduled_date = None;
    let mut recurrence = None;
    let mut done_date = None;
    let mut start_date = None;
    let mut priority = Priority::None;
    let mut tags = Vec::new();

    let mut chars = rest.chars().peekable();
    let mut current_word = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '📅' => {
                description.push_str(current_word.trim());
                current_word.clear();
                due_date = parse_date_from_iter(&mut chars);
            }
            '🛫' => {
                description.push_str(current_word.trim());
                current_word.clear();
                start_date = parse_date_from_iter(&mut chars);
            }
            '🔁' => {
                description.push_str(current_word.trim());
                current_word.clear();
                recurrence = Some(parse_until_emoji_or_end(&mut chars));
            }
            '✅' => {
                description.push_str(current_word.trim());
                current_word.clear();
                done_date = parse_date_from_iter(&mut chars);
            }
            '⏳' => {
                description.push_str(current_word.trim());
                current_word.clear();
                scheduled_date = parse_date_from_iter(&mut chars);
            }
            '⏫' | '🔼' | '🔽' | '⏬' => {
                description.push_str(current_word.trim());
                current_word.clear();
                priority = Priority::from_emoji(&ch.to_string()).unwrap_or(Priority::None);
            }
            '#' => {
                description.push_str(current_word.trim());
                current_word.clear();
                let tag = parse_tag(&mut chars);
                if !tag.is_empty() {
                    tags.push(tag);
                }
            }
            _ => {
                current_word.push(ch);
            }
        }
    }

    if !current_word.trim().is_empty() {
        if !description.is_empty() {
            description.push(' ');
        }
        description.push_str(current_word.trim());
    }

    Some(Task {
        description: description.trim().to_string(),
        status,
        priority,
        due_date,
        scheduled_date,
        recurrence,
        done_date,
        start_date,
        tags,
        source_file: source_file.clone(),
        line_number,
    })
}

fn parse_date_from_iter(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<NaiveDate> {
    let mut date_str = String::new();

    // Skip whitespace
    while chars.peek() == Some(&' ') {
        chars.next();
    }

    // Read date (YYYY-MM-DD or YYYY/MM/DD)
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() || ch == '-' || ch == '/' {
            date_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    // Try parsing with different formats
    NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(&date_str, "%Y/%m/%d"))
        .ok()
}

fn parse_until_emoji_or_end(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut text = String::new();

    // Skip whitespace
    while chars.peek() == Some(&' ') {
        chars.next();
    }

    while let Some(&ch) = chars.peek() {
        if is_emoji(ch) || ch == '#' {
            break;
        }
        text.push(ch);
        chars.next();
    }

    text.trim().to_string()
}

fn parse_tag(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut tag = String::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '/' {
            tag.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    tag
}

fn is_emoji(ch: char) -> bool {
    // Check for common task emojis
    matches!(ch, '📅' | '🛫' | '🔁' | '✅' | '⏳' | '⏫' | '🔼' | '🔽' | '⏬')
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `nix-shell --run "cargo test --lib"`
Expected: All 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/task/parser.rs tests/fixtures/
git commit -m "feat: add markdown task parser with emoji metadata support"
```

---

## Task 4: Config Module

**Files:**
- Create: `src/config.rs`

- [ ] **Step 1: Write config tests**

```rust
// src/config.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_valid_config() {
        let toml = r#"
[workspace]
path = "/home/user/vault"

[defaults]
view = "All Tasks"

[theme]
colors = "dark"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.workspace.path, "/home/user/vault");
        assert_eq!(config.defaults.view, "All Tasks");
        assert_eq!(config.theme.colors, "dark");
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[workspace]
path = "/tmp/vault"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.workspace.path, "/tmp/vault");
        assert_eq!(config.defaults.view, "All Tasks"); // default
        assert_eq!(config.theme.colors, "dark"); // default
    }

    #[test]
    fn test_config_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[workspace]\npath = \"/tmp/test\"").unwrap();

        let config = Config::from_file(file.path()).unwrap();
        assert_eq!(config.workspace.path, "/tmp/test");
    }

    #[test]
    fn test_config_from_missing_file() {
        let result = Config::from_file(std::path::Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix-shell --run "cargo test --lib"`
Expected: FAIL with "Config not found"

- [ ] **Step 3: Implement config module**

```rust
// src/config.rs
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_view")]
    pub view: String,
}

#[derive(Debug, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_colors")]
    pub colors: String,
}

fn default_view() -> String {
    "All Tasks".to_string()
}

fn default_colors() -> String {
    "dark".to_string()
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            view: default_view(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            colors: default_colors(),
        }
    }
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Try ./config.toml first
        if let Ok(config) = Config::from_file(Path::new("config.toml")) {
            return Ok(config);
        }

        // Fall back to ~/.config/taskboard/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("taskboard").join("config.toml");
            if let Ok(config) = Config::from_file(&path) {
                return Ok(config);
            }
        }

        Err("No config file found".into())
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix-shell --run "cargo test --lib"`
Expected: All 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add config module with TOML parsing"
```

---

## Task 5: Vault Walker

**Files:**
- Create: `src/vault.rs`

- [ ] **Step 1: Write vault tests**

```rust
// src/vault.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_vault() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create markdown files
        fs::write(dir.path().join("tasks.md"), "- [ ] Task 1\n- [x] Task 2").unwrap();
        fs::write(dir.path().join("notes.md"), "# Notes\nNot a task").unwrap();

        // Create subdirectory with tasks
        fs::create_dir(dir.path().join("projects")).unwrap();
        fs::write(dir.path().join("projects/todo.md"), "- [ ] Project task").unwrap();

        // Create files that should be skipped
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git/config"), "git config").unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/pkg.js"), "code").unwrap();
        fs::write(dir.path().join("image.png"), "binary").unwrap();

        dir
    }

    #[test]
    fn test_find_markdown_files() {
        let dir = create_test_vault();
        let files = find_markdown_files(dir.path());

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.ends_with("tasks.md")));
        assert!(files.iter().any(|f| f.ends_with("notes.md")));
        assert!(files.iter().any(|f| f.ends_with("todo.md")));
    }

    #[test]
    fn test_skip_git_and_node_modules() {
        let dir = create_test_vault();
        let files = find_markdown_files(dir.path());

        assert!(!files.iter().any(|f| f.to_string_lossy().contains(".git")));
        assert!(!files.iter().any(|f| f.to_string_lossy().contains("node_modules")));
    }

    #[test]
    fn test_skip_non_markdown() {
        let dir = create_test_vault();
        let files = find_markdown_files(dir.path());

        assert!(!files.iter().any(|f| f.ends_with("image.png")));
        assert!(!files.iter().any(|f| f.ends_with("pkg.js")));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix-shell --run "cargo test --lib"`
Expected: FAIL with "find_markdown_files not found"

- [ ] **Step 3: Implement vault walker**

```rust
// src/vault.rs
use std::path::{Path, PathBuf};

pub fn find_markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    find_markdown_files_recursive(root, &mut files);
    files
}

fn find_markdown_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden directories and common non-project dirs
        if path.is_dir() {
            if name_str.starts_with('.') || name_str == "node_modules" || name_str == "target" {
                continue;
            }
            find_markdown_files_recursive(&path, files);
        } else if path.is_file() {
            if name_str.ends_with(".md") || name_str.ends_with(".markdown") {
                files.push(path);
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix-shell --run "cargo test --lib"`
Expected: All 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/vault.rs
git commit -m "feat: add vault walker for markdown file discovery"
```

---

## Task 6: Query Engine

**Files:**
- Create: `src/task/query.rs`

- [ ] **Step 1: Write query engine tests**

```rust
// src/task/query.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Task, TaskStatus, Priority};
    use chrono::NaiveDate;
    use std::path::PathBuf;

    fn sample_tasks() -> Vec<Task> {
        vec![
            Task {
                description: "Buy groceries".to_string(),
                status: TaskStatus::Todo,
                priority: Priority::None,
                due_date: Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
                scheduled_date: None,
                recurrence: None,
                done_date: None,
                start_date: None,
                tags: vec!["personal".to_string()],
                source_file: PathBuf::from("tasks.md"),
                line_number: 1,
            },
            Task {
                description: "Review PR".to_string(),
                status: TaskStatus::Done,
                priority: Priority::High,
                due_date: Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
                scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 6, 9).unwrap()),
                recurrence: None,
                done_date: Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
                start_date: None,
                tags: vec!["work".to_string()],
                source_file: PathBuf::from("work.md"),
                line_number: 5,
            },
            Task {
                description: "Fix bug".to_string(),
                status: TaskStatus::Todo,
                priority: Priority::Medium,
                due_date: None,
                scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()),
                recurrence: Some("every week".to_string()),
                done_date: None,
                start_date: None,
                tags: vec!["work".to_string(), "urgent".to_string()],
                source_file: PathBuf::from("bugs.md"),
                line_number: 10,
            },
        ]
    }

    #[test]
    fn test_filter_done() {
        let tasks = sample_tasks();
        let result = execute_query("done", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Review PR");
    }

    #[test]
    fn test_filter_not_done() {
        let tasks = sample_tasks();
        let result = execute_query("not done", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_includes() {
        let tasks = sample_tasks();
        let result = execute_query("includes bug", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_filter_tag() {
        let tasks = sample_tasks();
        let result = execute_query("tag personal", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Buy groceries");
    }

    #[test]
    fn test_filter_priority_high() {
        let tasks = sample_tasks();
        let result = execute_query("priority is high", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Review PR");
    }

    #[test]
    fn test_filter_has_recurrence() {
        let tasks = sample_tasks();
        let result = execute_query("has recurrence", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_sort_by_priority() {
        let tasks = sample_tasks();
        let result = execute_query("sort by priority", &tasks).unwrap();
        assert_eq!(result[0].priority, Priority::High);
        assert_eq!(result[1].priority, Priority::Medium);
        assert_eq!(result[2].priority, Priority::None);
    }

    #[test]
    fn test_combined_query() {
        let tasks = sample_tasks();
        let result = execute_query("not done tag work sort by priority", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_empty_query() {
        let tasks = sample_tasks();
        let result = execute_query("", &tasks).unwrap();
        assert_eq!(result.len(), 3);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix-shell --run "cargo test --lib"`
Expected: FAIL with "execute_query not found"

- [ ] **Step 3: Implement query engine**

```rust
// src/task/query.rs
use super::{Task, TaskStatus, Priority};
use chrono::NaiveDate;

#[derive(Debug)]
pub enum Filter {
    Done,
    NotDone,
    Includes(String),
    DescriptionIncludes(String),
    Tag(String),
    Folder(String),
    DueBefore(NaiveDate),
    DueAfter(NaiveDate),
    DueOn(NaiveDate),
    ScheduledBefore(NaiveDate),
    ScheduledAfter(NaiveDate),
    ScheduledOn(NaiveDate),
    HappensBefore(NaiveDate),
    HappensAfter(NaiveDate),
    HappensOn(NaiveDate),
    PriorityAbove(Priority),
    PriorityBelow(Priority),
    PriorityIs(Priority),
    HasRecurrence,
    RecurrenceIncludes(String),
    Limit(usize),
}

#[derive(Debug)]
pub enum SortField {
    Due,
    Scheduled,
    Priority,
    Description,
    Tag,
    Folder,
    Done,
    Created,
}

#[derive(Debug)]
pub struct Query {
    pub filters: Vec<Filter>,
    pub sort_by: Option<SortField>,
    pub group_by: Option<SortField>,
}

pub fn execute_query(query_str: &str, tasks: &[Task]) -> Result<Vec<Task>, String> {
    let query = parse_query(query_str)?;
    let mut result: Vec<Task> = tasks.to_vec();

    // Apply filters
    for filter in &query.filters {
        result = result.into_iter().filter(|t| matches_filter(t, filter)).collect();
    }

    // Apply sorting
    if let Some(sort_field) = &query.sort_by {
        result.sort_by(|a, b| compare_tasks(a, b, sort_field));
    }

    Ok(result)
}

fn parse_query(query_str: &str) -> Result<Query, String> {
    let mut filters = Vec::new();
    let mut sort_by = None;
    let mut group_by = None;

    let tokens: Vec<&str> = query_str.split_whitespace().collect();
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i] {
            "done" => {
                filters.push(Filter::Done);
                i += 1;
            }
            "not" if i + 1 < tokens.len() && tokens[i + 1] == "done" => {
                filters.push(Filter::NotDone);
                i += 2;
            }
            "includes" if i + 1 < tokens.len() => {
                filters.push(Filter::Includes(tokens[i + 1].to_string()));
                i += 2;
            }
            "description" if i + 1 < tokens.len() && tokens[i + 1] == "includes" && i + 2 < tokens.len() => {
                filters.push(Filter::DescriptionIncludes(tokens[i + 2].to_string()));
                i += 3;
            }
            "tag" if i + 1 < tokens.len() => {
                filters.push(Filter::Tag(tokens[i + 1].to_string()));
                i += 2;
            }
            "folder" if i + 1 < tokens.len() => {
                filters.push(Filter::Folder(tokens[i + 1].to_string()));
                i += 2;
            }
            "due" if i + 2 < tokens.len() => {
                let date = parse_date(tokens[i + 2])?;
                match tokens[i + 1] {
                    "before" => filters.push(Filter::DueBefore(date)),
                    "after" => filters.push(Filter::DueAfter(date)),
                    "on" => filters.push(Filter::DueOn(date)),
                    _ => return Err(format!("Unknown due filter: {}", tokens[i + 1])),
                }
                i += 3;
            }
            "scheduled" if i + 2 < tokens.len() => {
                let date = parse_date(tokens[i + 2])?;
                match tokens[i + 1] {
                    "before" => filters.push(Filter::ScheduledBefore(date)),
                    "after" => filters.push(Filter::ScheduledAfter(date)),
                    "on" => filters.push(Filter::ScheduledOn(date)),
                    _ => return Err(format!("Unknown scheduled filter: {}", tokens[i + 1])),
                }
                i += 3;
            }
            "happens" if i + 2 < tokens.len() => {
                let date = parse_date(tokens[i + 2])?;
                match tokens[i + 1] {
                    "before" => filters.push(Filter::HappensBefore(date)),
                    "after" => filters.push(Filter::HappensAfter(date)),
                    "on" => filters.push(Filter::HappensOn(date)),
                    _ => return Err(format!("Unknown happens filter: {}", tokens[i + 1])),
                }
                i += 3;
            }
            "priority" if i + 2 < tokens.len() && tokens[i + 1] == "is" => {
                let priority = parse_priority(tokens[i + 2])?;
                filters.push(Filter::PriorityIs(priority));
                i += 3;
            }
            "has" if i + 1 < tokens.len() && tokens[i + 1] == "recurrence" => {
                filters.push(Filter::HasRecurrence);
                i += 2;
            }
            "recurrence" if i + 1 < tokens.len() && tokens[i + 1] == "includes" && i + 2 < tokens.len() => {
                filters.push(Filter::RecurrenceIncludes(tokens[i + 2].to_string()));
                i += 3;
            }
            "limit" if i + 1 < tokens.len() => {
                let n = tokens[i + 1].parse::<usize>().map_err(|_| "Invalid limit")?;
                filters.push(Filter::Limit(n));
                i += 2;
            }
            "sort" if i + 2 < tokens.len() && tokens[i + 1] == "by" => {
                sort_by = Some(parse_sort_field(tokens[i + 2])?);
                i += 3;
            }
            "group" if i + 2 < tokens.len() && tokens[i + 1] == "by" => {
                group_by = Some(parse_sort_field(tokens[i + 2])?);
                i += 3;
            }
            _ => return Err(format!("Unknown query token: {}", tokens[i])),
        }
    }

    Ok(Query {
        filters,
        sort_by,
        group_by,
    })
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    // Handle relative dates
    let today = chrono::Local::now().date_naive();
    match s {
        "today" => Ok(today),
        "tomorrow" => Ok(today + chrono::Duration::days(1)),
        "yesterday" => Ok(today - chrono::Duration::days(1)),
        "next sunday" => {
            let days_until_sunday = (7 - today.weekday().num_days_from_monday()) % 7;
            Ok(today + chrono::Duration::days(days_until_sunday as i64))
        }
        _ => {
            // Try parsing as YYYY-MM-DD
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date: {}", s))
        }
    }
}

fn parse_priority(s: &str) -> Result<Priority, String> {
    match s {
        "high" => Ok(Priority::High),
        "medium" => Ok(Priority::Medium),
        "low" => Ok(Priority::Low),
        "none" => Ok(Priority::None),
        _ => Err(format!("Unknown priority: {}", s)),
    }
}

fn parse_sort_field(s: &str) -> Result<SortField, String> {
    match s {
        "due" => Ok(SortField::Due),
        "scheduled" => Ok(SortField::Scheduled),
        "priority" => Ok(SortField::Priority),
        "description" => Ok(SortField::Description),
        "tag" => Ok(SortField::Tag),
        "folder" => Ok(SortField::Folder),
        "done" => Ok(SortField::Done),
        "created" => Ok(SortField::Created),
        _ => Err(format!("Unknown sort field: {}", s)),
    }
}

fn matches_filter(task: &Task, filter: &Filter) -> bool {
    match filter {
        Filter::Done => task.status == TaskStatus::Done,
        Filter::NotDone => task.status == TaskStatus::Todo,
        Filter::Includes(text) => task.description.to_lowercase().contains(&text.to_lowercase()),
        Filter::DescriptionIncludes(text) => task.description.to_lowercase().contains(&text.to_lowercase()),
        Filter::Tag(tag) => task.tags.contains(&tag.to_string()),
        Filter::Folder(folder) => task.source_file.to_string_lossy().contains(folder.as_str()),
        Filter::DueBefore(date) => task.due_date.map_or(false, |d| d < *date),
        Filter::DueAfter(date) => task.due_date.map_or(false, |d| d > *date),
        Filter::DueOn(date) => task.due_date == Some(*date),
        Filter::ScheduledBefore(date) => task.scheduled_date.map_or(false, |d| d < *date),
        Filter::ScheduledAfter(date) => task.scheduled_date.map_or(false, |d| d > *date),
        Filter::ScheduledOn(date) => task.scheduled_date == Some(*date),
        Filter::HappensBefore(date) => {
            task.due_date.map_or(false, |d| d < *date) ||
            task.scheduled_date.map_or(false, |d| d < *date)
        }
        Filter::HappensAfter(date) => {
            task.due_date.map_or(false, |d| d > *date) ||
            task.scheduled_date.map_or(false, |d| d > *date)
        }
        Filter::HappensOn(date) => {
            task.due_date == Some(*date) || task.scheduled_date == Some(*date)
        }
        Filter::PriorityAbove(p) => task.priority > *p,
        Filter::PriorityBelow(p) => task.priority < *p,
        Filter::PriorityIs(p) => task.priority == *p,
        Filter::HasRecurrence => task.recurrence.is_some(),
        Filter::RecurrenceIncludes(text) => {
            task.recurrence.as_ref().map_or(false, |r| r.to_lowercase().contains(&text.to_lowercase()))
        }
        Filter::Limit(_) => true, // Applied after filtering
    }
}

fn compare_tasks(a: &Task, b: &Task, field: &SortField) -> std::cmp::Ordering {
    match field {
        SortField::Due => a.due_date.cmp(&b.due_date),
        SortField::Scheduled => a.scheduled_date.cmp(&b.scheduled_date),
        SortField::Priority => a.priority.cmp(&b.priority),
        SortField::Description => a.description.cmp(&b.description),
        SortField::Tag => a.tags.first().cmp(&b.tags.first()),
        SortField::Folder => a.source_file.cmp(&b.source_file),
        SortField::Done => a.status.cmp(&b.status),
        SortField::Created => a.line_number.cmp(&b.line_number),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix-shell --run "cargo test --lib"`
Expected: All 9 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/task/query.rs
git commit -m "feat: add query engine with filters, sorting, and grouping"
```

---

## Task 7: Views and Storage

**Files:**
- Create: `src/view.rs`
- Create: `src/storage.rs`

- [ ] **Step 1: Write view tests**

```rust
// src/view.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_creation() {
        let view = View::new("My View", "not done tag work", "priority", "due");
        assert_eq!(view.name, "My View");
        assert_eq!(view.query, "not done tag work");
        assert_eq!(view.sort_by, "priority");
        assert_eq!(view.group_by, "due");
    }

    #[test]
    fn test_default_view() {
        let view = View::default();
        assert_eq!(view.name, "All Tasks");
        assert_eq!(view.query, "");
        assert_eq!(view.sort_by, "due_date");
        assert_eq!(view.group_by, "");
    }
}
```

- [ ] **Step 2: Write storage tests**

```rust
// src/storage.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_views() {
        let file = NamedTempFile::new().unwrap();
        let views = vec![
            View::new("View 1", "not done", "priority", ""),
            View::new("View 2", "tag work", "due", "folder"),
        ];

        save_views(&views, file.path()).unwrap();
        let loaded = load_views(file.path()).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "View 1");
        assert_eq!(loaded[1].name, "View 2");
    }

    #[test]
    fn test_load_missing_file() {
        let views = load_views(std::path::Path::new("/nonexistent/views.toml")).unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "All Tasks"); // default view
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `nix-shell --run "cargo test --lib"`
Expected: FAIL with "View not found"

- [ ] **Step 4: Implement view and storage**

```rust
// src/view.rs
#[derive(Debug, Clone)]
pub struct View {
    pub name: String,
    pub query: String,
    pub sort_by: String,
    pub group_by: String,
}

impl View {
    pub fn new(name: &str, query: &str, sort_by: &str, group_by: &str) -> Self {
        Self {
            name: name.to_string(),
            query: query.to_string(),
            sort_by: sort_by.to_string(),
            group_by: group_by.to_string(),
        }
    }
}

impl Default for View {
    fn default() -> Self {
        Self::new("All Tasks", "", "due_date", "")
    }
}
```

```rust
// src/storage.rs
use crate::view::View;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct ViewsFile {
    views: Vec<ViewSerde>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ViewSerde {
    name: String,
    query: String,
    #[serde(default)]
    sort_by: String,
    #[serde(default)]
    group_by: String,
}

pub fn load_views(path: &Path) -> Result<Vec<View>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(vec![View::default()]);
    }

    let content = std::fs::read_to_string(path)?;
    let file: ViewsFile = toml::from_str(&content)?;

    let views = file.views.into_iter().map(|v| View {
        name: v.name,
        query: v.query,
        sort_by: v.sort_by,
        group_by: v.group_by,
    }).collect();

    Ok(views)
}

pub fn save_views(views: &[View], path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = ViewsFile {
        views: views.iter().map(|v| ViewSerde {
            name: v.name.clone(),
            query: v.query.clone(),
            sort_by: v.sort_by.clone(),
            group_by: v.group_by.clone(),
        }).collect(),
    };

    let content = toml::to_string_pretty(&file)?;
    std::fs::write(path, content)?;
    Ok(())
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `nix-shell --run "cargo test --lib"`
Expected: All 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/view.rs src/storage.rs
git commit -m "feat: add views and storage for saved queries"
```

---

## Task 8: UI Foundation (App State and Event Loop)

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/main.rs` (update)

- [ ] **Step 1: Implement basic app state and event loop**

```rust
// src/ui/mod.rs
pub mod task_list;
pub mod modal;
pub mod command;
pub mod theme;

use crate::config::Config;
use crate::task::Task;
use crate::view::View;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

pub struct App {
    pub tasks: Vec<Task>,
    pub filtered_tasks: Vec<Task>,
    pub selected_index: usize,
    pub views: Vec<View>,
    pub current_view: View,
    pub config: Config,
    pub should_quit: bool,
    pub show_help: bool,
}

impl App {
    pub fn new(config: Config, tasks: Vec<Task>, views: Vec<View>) -> Self {
        let current_view = views.first().cloned().unwrap_or_default();
        Self {
            tasks,
            filtered_tasks: Vec::new(),
            selected_index: 0,
            views,
            current_view,
            config,
            should_quit: false,
            show_help: false,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        loop {
            self.update_filtered_tasks();
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => self.selected_index = 0,
            KeyCode::Char('G') => self.selected_index = self.filtered_tasks.len().saturating_sub(1),
            KeyCode::Char('x') => self.toggle_done(),
            KeyCode::Char('p') => self.cycle_priority(),
            KeyCode::Char('d') => self.set_due_date_today(),
            KeyCode::Char('D') => self.set_due_date_tomorrow(),
            KeyCode::Char('s') => self.set_scheduled_today(),
            KeyCode::Char('S') => self.set_scheduled_tomorrow(),
            KeyCode::Char('b') => self.bump_scheduled(),
            _ => {}
        }
    }

    fn move_down(&mut self) {
        if !self.filtered_tasks.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_tasks.len();
        }
    }

    fn move_up(&mut self) {
        if !self.filtered_tasks.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_tasks.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    fn toggle_done(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.status.cycle();
        }
    }

    fn cycle_priority(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.priority.cycle();
        }
    }

    fn set_due_date_today(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.due_date = Some(chrono::Local::now().date_naive());
        }
    }

    fn set_due_date_tomorrow(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.due_date = Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
        }
    }

    fn set_scheduled_today(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.scheduled_date = Some(chrono::Local::now().date_naive());
        }
    }

    fn set_scheduled_tomorrow(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.scheduled_date = Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
        }
    }

    fn bump_scheduled(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            let date = task.scheduled_date.unwrap_or_else(|| chrono::Local::now().date_naive());
            task.scheduled_date = Some(date + chrono::Duration::days(1));
        }
    }

    fn update_filtered_tasks(&mut self) {
        self.filtered_tasks = crate::task::query::execute_query(
            &self.current_view.query,
            &self.tasks,
        ).unwrap_or_default();
    }

    fn draw(&self, frame: &mut ratatui::Frame) {
        task_list::draw(frame, self);
    }
}
```

- [ ] **Step 2: Update main.rs**

```rust
// src/main.rs
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

    let mut terminal = ratatui::init();
    let mut app = ui::App::new(config, all_tasks, views);
    app.run(&mut terminal)?;
    ratatui::restore();

    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `nix-shell --run "cargo build"`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs src/main.rs
git commit -m "feat: add basic app state and event loop with keybindings"
```

---

## Task 9: Task List Widget

**Files:**
- Create: `src/ui/task_list.rs`

- [ ] **Step 1: Implement task list rendering**

```rust
// src/ui/task_list.rs
use crate::ui::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_task_list(frame, app, chunks[0]);
    draw_status_bar(frame, app, chunks[1]);
}

fn draw_task_list(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let width = area.width as usize;

    let items: Vec<ListItem> = app
        .filtered_tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let status = if task.status == crate::task::TaskStatus::Done {
                "[x]"
            } else {
                "[ ]"
            };

            let priority = task.priority.to_emoji();
            let priority_str = if priority.is_empty() {
                String::new()
            } else {
                format!("{} ", priority)
            };

            let due = task
                .due_date
                .map(|d| {
                    if width < 80 {
                        format!(" 📅{}", d.format("%m-%d"))
                    } else {
                        format!(" 📅 {}", d.format("%Y-%m-%d"))
                    }
                })
                .unwrap_or_default();

            let scheduled = task
                .scheduled_date
                .map(|d| {
                    if width < 80 {
                        format!(" ⏳{}", d.format("%m-%d"))
                    } else {
                        format!(" ⏳ {}", d.format("%Y-%m-%d"))
                    }
                })
                .unwrap_or_default();

            let source = if width >= 120 {
                format!(" {}", task.source_file.display())
            } else {
                String::new()
            };

            let line = if i == app.selected_index {
                Line::from(vec![
                    Span::styled(
                        format!("{} {}{}{}{}{}", status, priority_str, task.description, due, scheduled, source),
                        Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(status, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::raw(&priority_str),
                    Span::raw(&task.description),
                    Span::styled(format!("{}{}{}", due, scheduled, source), Style::default().fg(Color::DarkGray)),
                ])
            };

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("TaskBoard - {}", app.current_view.name)),
    );

    frame.render_widget(list, area);
}

fn draw_status_bar(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let total = app.filtered_tasks.len();
    let done = app
        .filtered_tasks
        .iter()
        .filter(|t| t.status == crate::task::TaskStatus::Done)
        .count();
    let open = total - done;

    let status = format!(
        "{} tasks ({} done, {} open) | {} | q:quit / /:search / ?:help",
        total, done, open, app.current_view.name
    );

    let paragraph = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `nix-shell --run "cargo build"`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add src/ui/task_list.rs
git commit -m "feat: add task list widget with responsive layout"
```

---

## Task 10: Integration Test

**Files:**
- Create: `tests/fixtures/large.md`
- Create: `tests/integration_test.rs`

- [ ] **Step 1: Generate large test fixture**

```bash
# Generate a file with 1000 tasks
python3 -c "
for i in range(1000):
    status = '[x]' if i % 3 == 0 else '[ ]'
    priority = ['⏫', '🔼', '🔽', '⏬', ''][i % 5]
    print(f'- {status} Task {i} {priority}')
" > tests/fixtures/large.md
```

- [ ] **Step 2: Write integration test**

```rust
// tests/integration_test.rs
use std::path::PathBuf;

#[test]
fn test_full_pipeline() {
    // Parse tasks from fixtures
    let basic_content = std::fs::read_to_string("tests/fixtures/basic.md").unwrap();
    let basic_tasks = taskboard::task::parser::parse_file(&basic_content, &PathBuf::from("tests/fixtures/basic.md"));

    let full_content = std::fs::read_to_string("tests/fixtures/full_metadata.md").unwrap();
    let full_tasks = taskboard::task::parser::parse_file(&full_content, &PathBuf::from("tests/fixtures/full_metadata.md"));

    let mut all_tasks = basic_tasks;
    all_tasks.extend(full_tasks);

    // Test queries
    let done_tasks = taskboard::task::query::execute_query("done", &all_tasks).unwrap();
    assert!(done_tasks.iter().all(|t| t.status == taskboard::task::TaskStatus::Done));

    let not_done = taskboard::task::query::execute_query("not done", &all_tasks).unwrap();
    assert!(not_done.iter().all(|t| t.status == taskboard::task::TaskStatus::Todo));

    let high_priority = taskboard::task::query::execute_query("priority is high", &all_tasks).unwrap();
    assert!(high_priority.iter().all(|t| t.priority == taskboard::task::Priority::High));

    // Test sorting
    let sorted = taskboard::task::query::execute_query("sort by priority", &all_tasks).unwrap();
    for i in 0..sorted.len() - 1 {
        assert!(sorted[i].priority <= sorted[i + 1].priority);
    }
}

#[test]
fn test_large_file_performance() {
    let start = std::time::Instant::now();
    let content = std::fs::read_to_string("tests/fixtures/large.md").unwrap();
    let tasks = taskboard::task::parser::parse_file(&content, &PathBuf::from("tests/fixtures/large.md"));
    let duration = start.elapsed();

    assert_eq!(tasks.len(), 1000);
    assert!(duration.as_millis() < 500, "Parsing took {}ms, expected < 500ms", duration.as_millis());
}
```

- [ ] **Step 3: Run integration tests**

Run: `nix-shell --run "cargo test --test integration_test"`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add tests/fixtures/large.md tests/integration_test.rs
git commit -m "test: add integration tests and performance benchmark"
```

---

## Task 11: Filesystem Watching

**Files:**
- Modify: `src/vault.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add notify dependency to Cargo.toml**

Already present in Cargo.toml (notify = "6").

- [ ] **Step 2: Implement filesystem watcher**

```rust
// Add to src/vault.rs
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc;
use std::time::Duration;

pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<Event>>,
}

impl FileWatcher {
    pub fn new(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            tx.send(res).ok();
        })?;

        watcher.watch(path, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    pub fn poll_changes(&self) -> Vec<std::path::PathBuf> {
        let mut changed_files = Vec::new();

        while let Ok(Ok(event)) = self.receiver.recv_timeout(Duration::from_millis(0)) {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    for path in event.paths {
                        if path.extension().map_or(false, |ext| ext == "md" || ext == "markdown") {
                            changed_files.push(path);
                        }
                    }
                }
                _ => {}
            }
        }

        changed_files
    }
}
```

- [ ] **Step 3: Integrate watcher into app**

Add to `App` struct in `src/ui/mod.rs`:

```rust
pub struct App {
    // ... existing fields ...
    pub file_watcher: Option<vault::FileWatcher>,
}
```

Add to event loop:

```rust
// In the run method, after event polling
if let Some(watcher) = &self.file_watcher {
    let changed = watcher.poll_changes();
    for path in changed {
        if let Ok(content) = std::fs::read_to_string(&path) {
            // Remove old tasks from this file
            self.tasks.retain(|t| t.source_file != path);
            // Parse and add new tasks
            let new_tasks = task::parser::parse_file(&content, &path);
            self.tasks.extend(new_tasks);
        }
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `nix-shell --run "cargo build"`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src/vault.rs src/ui/mod.rs
git commit -m "feat: add filesystem watching with notify crate"
```

---

## Task 12: Help Overlay

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add help overlay rendering**

Add to `draw` method in `src/ui/mod.rs`:

```rust
fn draw(&self, frame: &mut ratatui::Frame) {
    task_list::draw(frame, self);

    if self.show_help {
        draw_help_overlay(frame);
    }
}

fn draw_help_overlay(frame: &mut ratatui::Frame) {
    use ratatui::layout::{Constraint, Direction, Layout, Rect};
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let area = frame.area();
    let popup_width = 50.min(area.width - 4);
    let popup_height = 20.min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from("Navigation:"),
        Line::from("  j/k, ↑/↓  Move up/down"),
        Line::from("  g/G       Jump to top/bottom"),
        Line::from(""),
        Line::from("Quick Actions:"),
        Line::from("  x         Toggle done"),
        Line::from("  p         Cycle priority"),
        Line::from("  d/D       Due date: today/tomorrow"),
        Line::from("  s/S       Scheduled: today/tomorrow"),
        Line::from("  b         Bump scheduled +1 day"),
        Line::from(""),
        Line::from("Views:"),
        Line::from("  /         Search/query"),
        Line::from("  v         Switch view"),
        Line::from("  V         Manage views"),
        Line::from(""),
        Line::from("  Enter     Edit task (modal)"),
        Line::from("  Ctrl+r    Rescan vault"),
        Line::from("  ?         Toggle help"),
        Line::from("  q         Quit"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .style(Style::default().bg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, popup_area);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `nix-shell --run "cargo build"`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat: add help overlay (? key)"
```

---

## Task 12: Final Polish and README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Run all tests**

Run: `nix-shell --run "cargo test"`
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: `nix-shell --run "cargo clippy -- -D warnings"`
Expected: No warnings

- [ ] **Step 3: Create README**

```markdown
# TaskBoard

A TUI app for managing Obsidian-style markdown tasks.

## Features

- Query and filter tasks using Obsidian Tasks syntax
- Save and manage reusable views
- Edit tasks with keyboard shortcuts or modal editor
- Real-time filesystem watching
- Responsive to terminal size

## Installation

```bash
cargo install --path .
```

## Usage

```bash
taskboard
```

## Configuration

Config file: `~/.config/taskboard/config.toml` or `./config.toml`

```toml
[workspace]
path = "/path/to/obsidian/vault"

[defaults]
view = "All Tasks"

[theme]
colors = "dark"
```

## Keybindings

| Key | Action |
|-----|--------|
| j/k | Move up/down |
| g/G | Jump to top/bottom |
| x | Toggle done |
| p | Cycle priority |
| d/D | Due date: today/tomorrow |
| s/S | Scheduled: today/tomorrow |
| b | Bump scheduled +1 day |
| / | Search/query |
| v | Switch view |
| V | Manage views |
| Enter | Edit task (modal) |
| Ctrl+r | Rescan vault |
| ? | Help |
| q | Quit |
```

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: add README with usage instructions"
```
