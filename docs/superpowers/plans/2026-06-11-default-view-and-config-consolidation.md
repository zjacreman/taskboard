# Default View & Config Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire up the unused `config.defaults.view` setting and consolidate `views.toml` into `config.toml`.

**Architecture:** Views move from a standalone `views.toml` into a `[[views]]` array-of-tables in `config.toml`. A new `ViewConfig` struct handles serialization. `Config::save()` writes back to disk. The view manager gets an `s` key to set the default view.

**Tech Stack:** Rust, serde (Serialize + Deserialize), toml crate (already in use)

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `src/config.rs` | Config parsing and serialization | Modify: add `ViewConfig`, `Serialize` derives, `Config::save()` |
| `src/storage.rs` | View persistence (standalone file) | Modify: remove view loading/saving code |
| `src/main.rs` | App bootstrap | Modify: load views from config, pass config_path |
| `src/ui/mod.rs` | App state and view manager | Modify: wire up default view, add `s` key, config save |
| `README.md` | Documentation | Modify: update config example |

---

### Task 1: Update config.rs — Add ViewConfig, Serialize, and Config::save()

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs` (inline tests)

- [ ] **Step 1: Add Serialize to imports and derives**

In `src/config.rs`, change the import and all struct derives:

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
```

Add `Serialize` to every struct derive:
- `Config`: `#[derive(Debug, Deserialize, Serialize)]`
- `WorkspaceConfig`: `#[derive(Debug, Deserialize, Serialize)]`
- `DefaultsConfig`: `#[derive(Debug, Deserialize, Serialize)]`
- `ThemeConfig`: `#[derive(Debug, Deserialize, Serialize)]`

Remove `#[allow(dead_code)]` from `Config` and `DefaultsConfig` and `ThemeConfig` (no longer needed once fields are used).

- [ ] **Step 2: Add ViewConfig struct and views field**

Add this struct after `ThemeConfig`:

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct ViewConfig {
    pub name: String,
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default)]
    pub group_by: String,
}

fn default_sort_by() -> String {
    "due_date".to_string()
}
```

Add the `views` field to `Config`:

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub views: Vec<ViewConfig>,
}
```

- [ ] **Step 3: Add Config::save() method**

Add this method to the `impl Config` block, after `load()`:

```rust
pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = toml::to_string_pretty(self)?;
    std::fs::write(path, content)?;
    Ok(())
}
```

- [ ] **Step 4: Add test for config with views**

Add this test to the `mod tests` block in `config.rs`:

```rust
#[test]
fn test_parse_config_with_views() {
    let toml = r#"
[workspace]
path = "/home/user/vault"

[defaults]
view = "Overdue"

[theme]
colors = "dark"

[[views]]
name = "All Tasks"
query = ""
sort_by = "due_date"
group_by = ""

[[views]]
name = "Overdue"
query = "due < today"
sort_by = "due_date"
group_by = ""
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.views.len(), 2);
    assert_eq!(config.views[0].name, "All Tasks");
    assert_eq!(config.views[1].name, "Overdue");
    assert_eq!(config.views[1].query, "due < today");
    assert_eq!(config.defaults.view, "Overdue");
}
```

- [ ] **Step 5: Add test for config without views (default empty vec)**

```rust
#[test]
fn test_parse_config_without_views() {
    let toml = r#"
[workspace]
path = "/tmp/vault"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.views.is_empty());
}
```

- [ ] **Step 6: Add test for config serialize roundtrip**

```rust
#[test]
fn test_config_save_and_reload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let config = Config {
        workspace: WorkspaceConfig { path: PathBuf::from("/tmp/vault") },
        defaults: DefaultsConfig { view: "My View".to_string() },
        theme: ThemeConfig::default(),
        views: vec![
            ViewConfig {
                name: "All Tasks".to_string(),
                query: String::new(),
                sort_by: "due_date".to_string(),
                group_by: String::new(),
            },
            ViewConfig {
                name: "My View".to_string(),
                query: "tag work".to_string(),
                sort_by: "priority".to_string(),
                group_by: String::new(),
            },
        ],
    };

    config.save(&path).unwrap();
    let loaded = Config::from_file(&path).unwrap();

    assert_eq!(loaded.defaults.view, "My View");
    assert_eq!(loaded.views.len(), 2);
    assert_eq!(loaded.views[1].name, "My View");
    assert_eq!(loaded.views[1].query, "tag work");
}
```

- [ ] **Step 7: Run config tests**

Run: `cargo test config::`
Expected: All tests pass (including new tests + existing tests)

- [ ] **Step 8: Commit**

```bash
git add src/config.rs
git commit -m "feat: add ViewConfig, Serialize derives, and Config::save()"
```

---

### Task 2: Update main.rs — Load views from config, pass config_path

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Remove views.toml loading code**

In `src/main.rs`, delete lines 97-101:

```rust
    let views_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("taskboard")
        .join("views.toml");
    let views = storage::load_views(&views_path)?;
```

Replace with:

```rust
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("taskboard")
        .join("config.toml");

    let views: Vec<crate::view::View> = config.views.iter().map(|v| {
        crate::view::View::new(&v.name, &v.query, &v.sort_by, &v.group_by)
    }).collect();
```

- [ ] **Step 2: Pass config_path to App::new()**

Change line 107 from:

```rust
    let mut app = ui::App::new(config, all_tasks, views);
```

to:

```rust
    let mut app = ui::App::new(config, all_tasks, views, config_path);
```

- [ ] **Step 3: Update the first-run config writing**

In the first-run block (lines 64-80), the hardcoded config string should include an empty `[[views]]` section. Change the `format!` to:

```rust
    let toml = format!(
        "[workspace]\npath = \"{}\"\n\n[defaults]\nview = \"All Tasks\"\n\n[theme]\ncolors = \"dark\"\n\n[[views]]\nname = \"All Tasks\"\nquery = \"\"\nsort_by = \"due_date\"\ngroup_by = \"\"\n",
        config.workspace.path.display()
    );
```

- [ ] **Step 4: Remove storage module import**

Remove `mod storage;` from line 6 of `src/main.rs`. The storage module will be emptied in the next task. Also remove the now-unused `use std::path::PathBuf;` if it's no longer needed (it's still used by `config_path`, so keep it).

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: Compilation may fail until Task 3 updates storage.rs and Task 4 updates ui/mod.rs. If so, skip verification and proceed to next tasks.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: load views from config, pass config_path to App"
```

---

### Task 3: Remove view code from storage.rs

**Files:**
- Modify: `src/storage.rs`

- [ ] **Step 1: Remove all view-related code from storage.rs**

Replace the entire contents of `src/storage.rs` with an empty module (or a comment). Since no other code uses storage.rs, the entire file can be emptied:

```rust
// Views are now stored in config.toml
```

- [ ] **Step 2: Verify no remaining references to storage functions**

Run: `cargo build 2>&1 | head -30`
Expected: If main.rs still has `mod storage;`, it compiles but the module is empty. Any remaining references to `storage::load_views` or `storage::save_views` will show as compile errors — fix them.

- [ ] **Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "refactor: remove view loading/saving from storage.rs"
```

---

### Task 4: Update ui/mod.rs — Wire up default view, add config_path, add 's' key

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add config_path field to App struct**

Change the `App` struct to replace `views_path` with `config_path`:

```rust
pub struct App {
    pub tasks: Vec<Task>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub views: Vec<View>,
    pub current_view: View,
    pub config: Config,
    pub workspace_path: Option<std::path::PathBuf>,
    pub config_path: std::path::PathBuf,
    // ... rest unchanged
}
```

- [ ] **Step 2: Update App::new() signature and default view logic**

Change `App::new()` to accept `config_path` and look up the default view by name:

```rust
pub fn new(config: Config, tasks: Vec<Task>, views: Vec<View>, config_path: std::path::PathBuf) -> Self {
    let current_view = views.iter()
        .find(|v| v.name == config.defaults.view)
        .or(views.first())
        .cloned()
        .unwrap_or_default();
    let workspace_path = Some(config.workspace.path.clone());
    let mut view_manager_state = ListState::default();
    if !views.is_empty() {
        view_manager_state.select(Some(0));
    }
    Self {
        tasks,
        filtered_indices: Vec::new(),
        selected_index: 0,
        views,
        current_view,
        config,
        workspace_path,
        config_path,
        // ... rest unchanged
    }
}
```

Remove the old `views_path` computation (lines 57-60).

- [ ] **Step 3: Add save_config() method**

Replace the existing `save_views()` method (lines 381-385) with:

```rust
fn save_config(&self) {
    if let Err(e) = self.config.save(&self.config_path) {
        log::warn!("Failed to save config: {}", e);
    }
}
```

- [ ] **Step 4: Update all save_views() call sites to save_config()**

Search for `self.save_views()` in `handle_view_manager_key()` and replace with `self.save_config()`. There are two call sites:
- After editing a view field (line 317)
- After deleting a view (line 364)

- [ ] **Step 5: Add 's' keybinding for set default**

In `handle_view_manager_key()`, add a new match arm after the `'d'` arm:

```rust
KeyCode::Char('s') => {
    if let Some(idx) = self.view_manager_state.selected() {
        if let Some(view) = self.views.get(idx) {
            self.config.defaults.view = view.name.clone();
            self.save_config();
        }
    }
}
```

- [ ] **Step 6: Update tests — sample_config() and test_app()**

Update `sample_config()` to include views:

```rust
fn sample_config() -> Config {
    Config {
        workspace: WorkspaceConfig { path: PathBuf::from(".") },
        defaults: DefaultsConfig::default(),
        theme: ThemeConfig::default(),
        views: vec![],
    }
}
```

Update `test_app()` to pass a config_path:

```rust
fn test_app(tasks: Vec<Task>, views: Vec<View>) -> App {
    let config = sample_config();
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let mut app = App::new(config, tasks, views, config_path);
    // Leak the dir so it doesn't get deleted during the test
    std::mem::forget(dir);
    app
}
```

Remove the old `app.views_path = ...` line.

- [ ] **Step 7: Add test for default view selection from config**

```rust
#[test]
fn test_default_view_from_config() {
    let tasks = sample_tasks();
    let views = vec![
        View::new("All Tasks", "", "", ""),
        View::new("Overdue", "due < today", "", ""),
    ];
    let mut config = sample_config();
    config.defaults.view = "Overdue".to_string();
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let app = App::new(config, tasks, views, config_path);
    std::mem::forget(dir);

    assert_eq!(app.current_view.name, "Overdue");
}
```

- [ ] **Step 8: Add test for default view fallback when name doesn't match**

```rust
#[test]
fn test_default_view_fallback_no_match() {
    let tasks = sample_tasks();
    let views = vec![
        View::new("View1", "not done", "", ""),
        View::new("View2", "done", "", ""),
    ];
    let mut config = sample_config();
    config.defaults.view = "Nonexistent".to_string();
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let app = App::new(config, tasks, views, config_path);
    std::mem::forget(dir);

    assert_eq!(app.current_view.name, "View1"); // falls back to first
}
```

- [ ] **Step 9: Add test for 's' key sets default**

```rust
#[test]
fn test_view_manager_set_default() {
    let tasks = sample_tasks();
    let views = vec![
        View::new("View1", "not done", "", ""),
        View::new("View2", "done", "", ""),
    ];
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    // Navigate to View2
    app.handle_view_manager_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    // Set as default
    app.handle_view_manager_key(key_event(KeyCode::Char('s'), KeyModifiers::NONE));

    assert_eq!(app.config.defaults.view, "View2");
}
```

- [ ] **Step 10: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 11: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat: wire up default view from config, add 's' to set default"
```

---

### Task 5: Update view manager drawing — Help line and default indicator

**Files:**
- Modify: `src/ui/mod.rs` (`draw_view_manager` function)

- [ ] **Step 1: Update the help line**

In `draw_view_manager()`, find the help line (around line 515):

```rust
let help_line = Line::from(Span::styled(
    "Enter: switch | e: edit | d: del | Esc: close",
    Style::default().fg(Color::Gray),
));
```

Change to:

```rust
let help_line = Line::from(Span::styled(
    "Enter: switch | e: edit | d: del | s: set default | Esc: close",
    Style::default().fg(Color::Gray),
));
```

- [ ] **Step 2: Add default indicator to view list items**

In the view list item iterator (around line 498-512), update the format string to show `(default)` for the default view:

```rust
let items: Vec<ListItem> = app
    .views
    .iter()
    .enumerate()
    .map(|(i, v)| {
        let style = if Some(i) == app.view_manager_state.selected() {
            Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
        } else if v.name == app.current_view.name {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if v.name == app.current_view.name { "* " } else { "  " };
        let suffix = if v.name == app.config.defaults.view { " (default)" } else { "" };
        ListItem::new(Line::from(format!("{}{}{}", prefix, v.name, suffix))).style(style)
    })
    .collect();
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat: show default indicator in view manager, update help line"
```

---

### Task 6: Update README config example

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the config example**

Replace the Configuration section (lines 25-38) with:

```markdown
## Configuration

Config file: `~/.config/taskboard/config.toml` or `./config.toml`

```toml
[workspace]
path = "/path/to/obsidian/vault"

[defaults]
view = "All Tasks"

[theme]
colors = "dark"

[[views]]
name = "All Tasks"
query = ""
sort_by = "due_date"
group_by = ""

[[views]]
name = "Overdue"
query = "due < today"
sort_by = "due_date"
group_by = ""
```
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update config example with [[views]] section"
```

---

### Task 7: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

- [ ] **Step 3: Build release**

Run: `cargo build --release`
Expected: Successful build
