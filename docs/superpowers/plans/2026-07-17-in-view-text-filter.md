# In-View Text Filter & View Manager Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `/` a transient in-view text filter (description + tags, live, bottom bar) and move all view add/edit into the view management modal, deleting the old query command palette.

**Architecture:** New `src/ui/filter.rs` module (matching + key handling + bottom-bar draw). View manager moves to `src/ui/view_manager.rs` and gains a 4-field `ViewForm` (name/query/sort_by/group_by) with Tab focus navigation. `src/ui/command.rs` (query palette, save-view flow) is deleted. Filter applies in `App::update_filtered_tasks` after the view query; it is never persisted.

**Tech Stack:** Rust 2021, ratatui, crossterm, tui-textarea. Build/test/lint always via `nix-shell --run "..."`.

**Spec:** `docs/superpowers/specs/2026-07-17-in-view-text-filter-design.md`

---

## Key facts about the existing code (read first)

- `App` lives in `src/ui/mod.rs`. Tests are in `#[cfg(test)] mod tests` at the bottom of the same file, using helpers: `key_event(code, modifiers)`, `sample_config()`, `sample_views()`, `test_app(tasks, views)`.
- `crate::test_helpers::sample_tasks()` returns 3 tasks:
  - `tasks[0]` "Fix bug", tags `["work", "urgent"]`, `bugs.md:10`
  - `tasks[1]` "Buy groceries", tags `["personal"]`, `tasks.md:1`
  - `tasks[2]` "Review PR" (Done), tags `["work"]`, `work.md:5`
- Tags are stored **without** the `#` prefix.
- `App::handle_key` routes in order: help → `show_modal` (modal.rs) → `search_textarea` (command.rs) → `show_view_manager` → main list keys. `/` currently calls `start_search()`.
- `App::update_filtered_tasks` currently: uses `search_textarea` lines if open else `current_view.query` → `execute_query` → sort by path+line → map to indices → clamp `selected_index`.
- `handle_view_manager_key` and `draw_view_manager` currently live in `src/ui/mod.rs`. The view manager's `e` key edits name only via `app.view_edit` + `ViewEditField` enum.
- `src/ui/command.rs` contains: `handle_key`, `submit_query`, save-view flow (`save_view_edit`, `save_view_confirm_overwrite`), `draw`, `draw_save_view`, `draw_overwrite_confirm`.
- Status bar is drawn in `src/ui/task_list.rs` `draw_status_bar`, a 1-line area at the bottom. Current hint text: `q:quit / /:search / ?:help`.
- tui-textarea: `TextArea::new(Vec<String>)`, `set_cursor_line_style(Style::default())` (hides line highlight), `move_cursor(CursorMove::End)`, `set_cursor_style(...)` (default cursor style is REVERSED; `Style::default()` makes it invisible). `textarea.input(key)` handles editing; bare `Enter` inserts a newline. Widget is rendered as `frame.render_widget(&textarea, area)`.
- `App::save_config(&mut self)` is a private method in `src/ui/mod.rs`; it IS callable from child modules (`crate::ui::filter`, `crate::ui::view_manager`).
- All `App` struct fields are `pub`.

---

### Task 1: Filter matching helper (new `src/ui/filter.rs`)

**Files:**
- Create: `src/ui/filter.rs`
- Modify: `src/ui/mod.rs:1-3` (module list)

- [ ] **Step 1: Write the failing tests + skeleton**

Create `src/ui/filter.rs`:

```rust
use crate::task::Task;

/// Returns true if the task matches the given filter text: case-insensitive
/// substring match on the description and on tags. A leading '#' in the
/// filter is ignored for tag matching. Empty/whitespace filter matches all.
pub fn matches_filter(task: &Task, filter: &str) -> bool {
    let needle = filter.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    if task.description.to_lowercase().contains(&needle) {
        return true;
    }
    let tag_needle = needle.trim_start_matches('#');
    task.tags.iter().any(|t| t.to_lowercase().contains(tag_needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::sample_tasks;

    #[test]
    fn test_matches_description_case_insensitive() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[0], "BUG"));
        assert!(!matches_filter(&tasks[1], "bug"));
    }

    #[test]
    fn test_matches_tag() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[1], "personal"));
        assert!(matches_filter(&tasks[0], "urgent"));
        assert!(!matches_filter(&tasks[1], "urgent"));
    }

    #[test]
    fn test_matches_tag_with_hash_prefix() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[0], "#work"));
        assert!(matches_filter(&tasks[2], "#WORK"));
    }

    #[test]
    fn test_empty_and_whitespace_matches_all() {
        let tasks = sample_tasks();
        for task in &tasks {
            assert!(matches_filter(task, ""));
            assert!(matches_filter(task, "   "));
        }
    }
}
```

In `src/ui/mod.rs`, add `filter` to the module list (lines 1-3):

```rust
pub mod task_list;
pub mod modal;
pub mod command;
pub mod filter;
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `nix-shell --run "cargo test ui::filter"`
Expected: PASS — `test result: ok. 4 passed`

(This module is pure logic; the implementation above is already the minimal correct one, so tests pass immediately. That is fine — the tests lock the matching semantics.)

- [ ] **Step 3: Commit**

```bash
git add src/ui/filter.rs src/ui/mod.rs
git commit -m "feat: add task text filter matching helper"
```

---

### Task 2: App filter state + filtering pipeline

**Files:**
- Modify: `src/ui/mod.rs` (App struct, `App::new`, `update_filtered_tasks`, tests)

- [ ] **Step 1: Write the failing tests**

Add to `#[cfg(test)] mod tests` in `src/ui/mod.rs`:

```rust
#[test]
fn test_filter_narrows_tasks() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.filter_text = "bug".to_string();
    app.update_filtered_tasks();

    assert_eq!(app.filtered_indices.len(), 1);
    assert_eq!(app.tasks[app.filtered_indices[0]].description, "Fix bug");
}

#[test]
fn test_filter_by_tag() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.filter_text = "#work".to_string();
    app.update_filtered_tasks();

    assert_eq!(app.filtered_indices.len(), 2);
}

#[test]
fn test_filter_empty_shows_all() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.filter_text = "   ".to_string();
    app.update_filtered_tasks();

    assert_eq!(app.filtered_indices.len(), 3);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix-shell --run "cargo test filter_"`
Expected: FAIL — compile error `no field filter_text on type App`

- [ ] **Step 3: Add state and pipeline code**

In `src/ui/mod.rs`, add to the `App` struct (after `status_message`):

```rust
    pub status_message: Option<String>,
    pub filter_text: String,
    pub filter_textarea: Option<TextArea<'static>>,
```

In `App::new`, add to the struct literal (after `status_message: None,`):

```rust
            status_message: None,
            filter_text: String::new(),
            filter_textarea: None,
```

In `update_filtered_tasks`, apply the filter before clamping. Replace:

```rust
        self.filtered_indices = result.iter()
            .filter_map(|t| {
                self.tasks.iter().position(|task| {
                    task.source_file == t.source_file && task.line_number == t.line_number
                })
            })
            .collect();

        // Clamp selected_index
```

with:

```rust
        self.filtered_indices = result.iter()
            .filter_map(|t| {
                self.tasks.iter().position(|task| {
                    task.source_file == t.source_file && task.line_number == t.line_number
                })
            })
            .collect();

        self.filtered_indices
            .retain(|&idx| filter::matches_filter(&self.tasks[idx], &self.filter_text));

        // Clamp selected_index
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix-shell --run "cargo test filter"`
Expected: PASS — all `filter` tests pass (7 total: 4 matching + 3 pipeline)

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat: apply text filter in task filtering pipeline"
```

---

### Task 3: Filter key handling, `/` rebind, lifecycle

**Files:**
- Modify: `src/ui/filter.rs` (add `handle_key`)
- Modify: `src/ui/mod.rs` (`handle_key` routing + main keys, `start_filter`, remove `start_search`, view-switch clear, tests)

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/ui/mod.rs`:

```rust
#[test]
fn test_slash_opens_filter() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);

    app.handle_key(key_event(KeyCode::Char('/'), KeyModifiers::NONE));
    assert!(app.filter_textarea.is_some());
}

#[test]
fn test_filter_typing_updates_live() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);

    app.handle_key(key_event(KeyCode::Char('/'), KeyModifiers::NONE));
    for c in "bug".chars() {
        app.handle_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    assert_eq!(app.filter_text, "bug");
    assert!(app.filter_textarea.is_some()); // still open
}

#[test]
fn test_filter_enter_keeps() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);

    app.handle_key(key_event(KeyCode::Char('/'), KeyModifiers::NONE));
    for c in "bug".chars() {
        app.handle_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.filter_textarea.is_none());
    assert_eq!(app.filter_text, "bug");

    app.update_filtered_tasks();
    assert_eq!(app.filtered_indices.len(), 1);
}

#[test]
fn test_filter_esc_clears() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);

    app.handle_key(key_event(KeyCode::Char('/'), KeyModifiers::NONE));
    for c in "bug".chars() {
        app.handle_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_key(key_event(KeyCode::Esc, KeyModifiers::NONE));

    assert!(app.filter_textarea.is_none());
    assert_eq!(app.filter_text, "");
}

#[test]
fn test_esc_in_main_list_clears_active_filter() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.filter_text = "bug".to_string();

    app.handle_key(key_event(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.filter_text, "");
}

#[test]
fn test_esc_in_main_list_no_filter_noop() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);

    // Must not panic or change state
    app.handle_key(key_event(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.filter_text, "");
    assert!(!app.should_quit);
}

#[test]
fn test_filter_reopens_prefilled() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.filter_text = "bug".to_string();

    app.handle_key(key_event(KeyCode::Char('/'), KeyModifiers::NONE));
    assert_eq!(app.filter_textarea.as_ref().unwrap().lines()[0], "bug");
}

#[test]
fn test_view_switch_clears_filter() {
    let tasks = sample_tasks();
    let views = vec![
        View::new("View1", "not done", "", ""),
        View::new("View2", "done", "", ""),
    ];
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;
    app.filter_text = "bug".to_string();

    app.handle_view_manager_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.current_view.name, "View2");
    assert_eq!(app.filter_text, "");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix-shell --run "cargo test -- ui::tests::test_slash ui::tests::test_filter ui::tests::test_esc ui::tests::test_view_switch"`
Expected: FAIL — the new tests compile (fields exist from Task 2) but fail on assertions: `/` still opens `start_search` so `filter_textarea` stays `None`, Esc does nothing, and view switch doesn't clear the filter

- [ ] **Step 3: Implement**

In `src/ui/filter.rs`, add below `matches_filter` (and update the `use` block):

```rust
use crate::task::Task;
use crate::ui::App;
use crossterm::event::{KeyCode, KeyEvent};

// ... matches_filter unchanged ...

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.filter_textarea.is_none() {
        return;
    }
    match key.code {
        KeyCode::Esc => {
            app.filter_textarea = None;
            app.filter_text.clear();
            app.dirty = true;
        }
        KeyCode::Enter => {
            app.filter_textarea = None;
        }
        _ => {
            if let Some(textarea) = &mut app.filter_textarea {
                textarea.input(key);
                app.filter_text = textarea.lines().join("\n");
                app.dirty = true;
            }
        }
    }
}
```

In `src/ui/mod.rs`:

1. Update the import at the top (`use tui_textarea::TextArea;`) to:

```rust
use tui_textarea::{CursorMove, TextArea};
```

2. In `App::handle_key`, add the filter routing between the modal branch and the (still present) search branch, and rebind `/`:

Replace:

```rust
        if self.show_modal {
            modal::handle_key(self, key);
            return;
        }
        if self.search_textarea.is_some() {
            command::handle_key(self, key);
            return;
        }
```

with:

```rust
        if self.show_modal {
            modal::handle_key(self, key);
            return;
        }
        if self.filter_textarea.is_some() {
            filter::handle_key(self, key);
            return;
        }
        if self.search_textarea.is_some() {
            command::handle_key(self, key);
            return;
        }
```

Replace in the main `match code`:

```rust
            KeyCode::Char('/') => self.start_search(),
```

with:

```rust
            KeyCode::Char('/') => self.start_filter(),
            KeyCode::Esc => {
                if !self.filter_text.is_empty() {
                    self.filter_text.clear();
                    self.dirty = true;
                }
            }
```

3. Replace the whole `start_search` method:

```rust
    fn start_search(&mut self) {
        let mut textarea = TextArea::new(self.current_view.query.lines().map(|l| l.to_string()).collect());
        textarea.set_cursor_line_style(ratatui::style::Style::default());
        self.search_textarea = Some(textarea);
    }
```

with:

```rust
    fn start_filter(&mut self) {
        let mut textarea = TextArea::new(vec![self.filter_text.clone()]);
        textarea.set_cursor_line_style(ratatui::style::Style::default());
        textarea.move_cursor(CursorMove::End);
        self.filter_textarea = Some(textarea);
    }
```

(`start_search` is deleted here because leaving it unused would fail clippy with `-D warnings`. The rest of command.rs is still wired and compiles; it becomes fully unreachable and is removed in Task 5.)

4. In `handle_view_manager_key`, clear the filter on view switch. Replace:

```rust
            KeyCode::Enter => {
                if let Some(idx) = self.view_manager_state.selected() {
                    if let Some(view) = self.views.get(idx) {
                        self.current_view = view.clone();
                        self.show_view_manager = false;
                        self.dirty = true;
                    }
                }
            }
```

with:

```rust
            KeyCode::Enter => {
                if let Some(idx) = self.view_manager_state.selected() {
                    if let Some(view) = self.views.get(idx) {
                        self.current_view = view.clone();
                        self.show_view_manager = false;
                        self.filter_text.clear();
                        self.dirty = true;
                    }
                }
            }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix-shell --run "cargo test"`
Expected: PASS — full suite green, including the 8 new lifecycle tests

- [ ] **Step 5: Commit**

```bash
git add src/ui/filter.rs src/ui/mod.rs
git commit -m "feat: bind / to live text filter with Enter/Esc lifecycle"
```

---

### Task 4: Filter bottom-bar UI + status bar indicator

**Files:**
- Modify: `src/ui/filter.rs` (add `draw`)
- Modify: `src/ui/mod.rs` (`App::draw`)
- Modify: `src/ui/task_list.rs` (`draw_status_bar`)

(Rendering has no unit tests; verification is build + clippy + the existing test suite staying green.)

- [ ] **Step 1: Implement filter bar draw**

In `src/ui/filter.rs`, extend imports and add `draw`:

```rust
use crate::task::Task;
use crate::ui::App;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Clear, Paragraph};
```

```rust
pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let Some(textarea) = &app.filter_textarea else { return };
    let area = frame.area();
    let bar = Rect::new(0, area.height.saturating_sub(1), area.width, 1);
    frame.render_widget(Clear, bar);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(bar);

    let label = Paragraph::new("/ ").style(Style::default().fg(Color::Yellow));
    frame.render_widget(label, chunks[0]);
    frame.render_widget(textarea, chunks[1]);
}
```

In `src/ui/mod.rs` `App::draw`, add after the `command::draw` block:

```rust
        if self.search_textarea.is_some() {
            command::draw(frame, self);
        }
        if self.filter_textarea.is_some() {
            filter::draw(frame, self);
        }
```

- [ ] **Step 2: Status bar indicator + hint text**

In `src/ui/task_list.rs` `draw_status_bar`, replace:

```rust
    let status = if let Some(msg) = &app.status_message {
        format!("{} | q:quit / /:search / ?:help", msg)
    } else {
        format!(
            "{} tasks ({} done, {} open) | {} | q:quit / /:search / ?:help",
            total, done, open, app.current_view.name
        )
    };
```

with:

```rust
    let filter_ind = if app.filter_text.trim().is_empty() {
        String::new()
    } else {
        format!(" | filter: \"{}\"", app.filter_text)
    };

    let status = if let Some(msg) = &app.status_message {
        format!("{} | q:quit / /:filter / ?:help", msg)
    } else {
        format!(
            "{} tasks ({} done, {} open) | {}{} | q:quit / /:filter / ?:help",
            total, done, open, app.current_view.name, filter_ind
        )
    };
```

- [ ] **Step 3: Verify build, tests, clippy**

Run: `nix-shell --run "cargo test"`
Expected: PASS — full suite green

Run: `nix-shell --run "cargo clippy -- -D warnings"`
Expected: PASS, no warnings

- [ ] **Step 4: Commit**

```bash
git add src/ui/filter.rs src/ui/mod.rs src/ui/task_list.rs
git commit -m "feat: render filter as bottom bar with status indicator"
```

---

### Task 5: Remove the command palette

**Files:**
- Delete: `src/ui/command.rs`
- Modify: `src/ui/mod.rs` (module list, App struct, `App::new`, `handle_key`, `App::draw`, `update_filtered_tasks`, help overlay, tests)

- [ ] **Step 1: Delete the module and strip all references**

1. Delete `src/ui/command.rs` (use `git rm src/ui/command.rs`).

2. In `src/ui/mod.rs` module list, remove:

```rust
pub mod command;
```

3. Remove from the `App` struct:

```rust
    pub search_textarea: Option<TextArea<'static>>,
    pub save_view_edit: Option<TextArea<'static>>,
    pub save_view_confirm_overwrite: Option<usize>,
```

4. Remove from `App::new`:

```rust
            search_textarea: None,
            save_view_edit: None,
            save_view_confirm_overwrite: None,
```

5. Remove from `App::handle_key`:

```rust
        if self.search_textarea.is_some() {
            command::handle_key(self, key);
            return;
        }
```

6. Remove from `App::draw`:

```rust
        if self.search_textarea.is_some() {
            command::draw(frame, self);
        }
```

7. In `update_filtered_tasks`, replace:

```rust
        let query = if let Some(textarea) = &self.search_textarea {
            textarea.lines().join("\n")
        } else {
            self.current_view.query.clone()
        };

        let mut result = crate::task::query::execute_query(&query, &self.tasks)
```

with:

```rust
        let mut result = crate::task::query::execute_query(&self.current_view.query, &self.tasks)
```

8. Remove the `test_search_alt_enter_submit` test from `mod tests`.

9. Update the help overlay in `draw_help_overlay`. Replace:

```rust
        Line::from("Views:"),
        Line::from("  /         Search/query"),
        Line::from("  v         Switch view"),
        Line::from("  V         Manage views (e:edit, d:del, s:default)"),
        Line::from(""),
```

with:

```rust
        Line::from("Views:"),
        Line::from("  /         Filter tasks (text)"),
        Line::from("  Esc       Clear active filter"),
        Line::from("  v         Manage views (a:add e:edit d:del s:default)"),
        Line::from(""),
```

(The old `V` line was stale — no such binding exists.)

- [ ] **Step 2: Verify full suite + clippy**

Run: `nix-shell --run "cargo test"`
Expected: PASS — full suite green (including `test_search_alt_enter_submit` gone)

Run: `nix-shell --run "cargo clippy -- -D warnings"`
Expected: PASS, no warnings (watch for unused imports left over)

- [ ] **Step 3: Commit**

```bash
git add -A src/ui/
git commit -m "refactor: remove query command palette in favor of text filter"
```

---

### Task 6: Move view manager into `src/ui/view_manager.rs` (pure refactor)

**Files:**
- Create: `src/ui/view_manager.rs`
- Modify: `src/ui/mod.rs` (module list, remove `handle_view_manager_key` body + `draw_view_manager`, keep wrapper)

No behavior change. All existing tests must keep passing unchanged.

- [ ] **Step 1: Move the code**

Create `src/ui/view_manager.rs` with the current contents of `handle_view_manager_key` (as a free function taking `app: &mut App`) and `draw_view_manager` (renamed `draw`), adjusting `self.` to `app.`:

```rust
use crate::ui::{App, ViewEditField};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use tui_textarea::TextArea;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let code = key.code;
    if app.view_edit.is_some() {
        match code {
            KeyCode::Esc => {
                app.view_edit = None;
            }
            KeyCode::Enter => {
                if let (Some(idx), Some(textarea)) = (app.view_manager_state.selected(), &app.view_edit) {
                    if let Some(view) = app.views.get_mut(idx) {
                        let text = textarea.lines()[0].clone();
                        match app.editing_view_field {
                            ViewEditField::Name => view.name = text,
                            ViewEditField::Query => view.query = text,
                            ViewEditField::SortBy => view.sort_by = text,
                            ViewEditField::GroupBy => view.group_by = text,
                        }
                    }
                }
                app.view_edit = None;
                app.save_config();
            }
            _ => {
                if let Some(textarea) = &mut app.view_edit {
                    textarea.input(key);
                }
            }
        }
        return;
    }

    match code {
        KeyCode::Esc => {
            app.show_view_manager = false;
        }
        KeyCode::Char('j') | KeyCode::Down
            if !app.views.is_empty() =>
        {
            let i = app.view_manager_state.selected().unwrap_or(0);
            let next = if i >= app.views.len() - 1 { 0 } else { i + 1 };
            app.view_manager_state.select(Some(next));
        }
        KeyCode::Char('k') | KeyCode::Up
            if !app.views.is_empty() =>
        {
            let i = app.view_manager_state.selected().unwrap_or(0);
            let prev = if i == 0 { app.views.len() - 1 } else { i - 1 };
            app.view_manager_state.select(Some(prev));
        }
        KeyCode::Char('e') => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    let mut textarea = TextArea::new(vec![view.name.clone()]);
                    textarea.set_cursor_line_style(ratatui::style::Style::default());
                    app.view_edit = Some(textarea);
                    app.editing_view_field = ViewEditField::Name;
                }
            }
        }
        KeyCode::Char('d')
            if app.views.len() > 1 =>
        {
            if let Some(idx) = app.view_manager_state.selected() {
                if idx < app.views.len() {
                    app.views.remove(idx);
                    let new_sel = if idx >= app.views.len() { app.views.len() - 1 } else { idx };
                    app.view_manager_state.select(Some(new_sel));
                    app.save_config();
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    app.config.defaults.view = view.name.clone();
                    app.save_config();
                }
            }
        }
        KeyCode::Enter => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    app.current_view = view.clone();
                    app.show_view_manager = false;
                    app.filter_text.clear();
                    app.dirty = true;
                }
            }
        }
        _ => {}
    }
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    let popup_width = 60.min(area.width - 4);
    let popup_height = (app.views.len() as u16 + 5).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    if app.view_edit.is_some() {
        let field_name = match app.editing_view_field {
            ViewEditField::Name => "Name",
            ViewEditField::Query => "Query",
            ViewEditField::SortBy => "Sort By",
            ViewEditField::GroupBy => "Group By",
        };

        let textarea = app.view_edit.as_ref().unwrap();

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Edit View")
            .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(block, popup_area);

        let inner = popup_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(inner);

        let view_name = app.views.get(app.view_manager_state.selected().unwrap_or(0))
            .map(|v| v.name.as_str())
            .unwrap_or("");
        let title = Paragraph::new(format!("Editing view: {}", view_name));
        frame.render_widget(title, chunks[0]);

        let label = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("{}: ", field_name), Style::default().fg(Color::Gray)),
            ])
        ]);
        frame.render_widget(label, chunks[1]);

        frame.render_widget(textarea, chunks[2]);

        let help = Paragraph::new(Span::styled(
            "Enter: save | Esc: cancel",
            Style::default().fg(Color::Gray),
        ));
        frame.render_widget(help, chunks[3]);
    } else {
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

        let help_line = Line::from(Span::styled(
            "Enter: switch | a: add | e: edit | d: del | s: set default | Esc: close",
            Style::default().fg(Color::Gray),
        ));

        let help_item = ListItem::new(help_line);
        let mut all_items = items;
        all_items.push(help_item);

        let list = List::new(all_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Manage Views")
                .style(Style::default().bg(Color::DarkGray)),
        );

        let mut state = ListState::default();
        state.select(app.view_manager_state.selected());
        frame.render_stateful_widget(list, popup_area, &mut state);
    }
}
```

In `src/ui/mod.rs`:

1. Add to module list: `pub mod view_manager;`
2. Delete the entire `draw_view_manager` free function.
3. Replace the `handle_view_manager_key` method body with a delegation wrapper (keeps existing tests compiling unchanged):

```rust
    pub fn handle_view_manager_key(&mut self, key: KeyEvent) {
        view_manager::handle_key(self, key);
    }
```

4. In `App::draw`, replace:

```rust
        if self.show_view_manager || self.view_edit.is_some() {
            draw_view_manager(frame, self);
        }
```

with:

```rust
        if self.show_view_manager || self.view_edit.is_some() {
            view_manager::draw(frame, self);
        }
```

- [ ] **Step 2: Verify tests + clippy**

Run: `nix-shell --run "cargo test"`
Expected: PASS — full suite green, unchanged behavior

Run: `nix-shell --run "cargo clippy -- -D warnings"`
Expected: PASS (fix any now-unused imports in mod.rs, e.g. `ListState` may still be needed for `view_manager_state` type — keep it)

- [ ] **Step 3: Commit**

```bash
git add src/ui/mod.rs src/ui/view_manager.rs
git commit -m "refactor: move view manager into its own module"
```

---

### Task 7: ViewForm — open, focus, cancel, rendering

**Files:**
- Modify: `src/ui/view_manager.rs` (ViewForm, key handling, form draw)
- Modify: `src/ui/mod.rs` (App struct: add `view_form`, remove `view_edit`/`editing_view_field`/`ViewEditField`; `App::new`; `App::draw` condition; tests)

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/ui/mod.rs`:

```rust
#[test]
fn test_view_form_add_opens_empty() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));

    let form = app.view_form.as_ref().expect("form should be open");
    assert_eq!(form.editing_index, None);
    assert_eq!(form.focus, 0);
    assert_eq!(form.fields[0].lines()[0], "");
    assert_eq!(form.fields[1].lines()[0], "");
}

#[test]
fn test_view_form_edit_opens_populated() {
    let tasks = sample_tasks();
    let views = vec![
        View::new("View1", "not done", "due_date", "tag"),
        View::new("View2", "done", "", ""),
    ];
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));

    let form = app.view_form.as_ref().expect("form should be open");
    assert_eq!(form.editing_index, Some(0));
    assert_eq!(form.fields[0].lines()[0], "View1");
    assert_eq!(form.fields[1].lines()[0], "not done");
    assert_eq!(form.fields[2].lines()[0], "due_date");
    assert_eq!(form.fields[3].lines()[0], "tag");
}

#[test]
fn test_view_form_focus_navigation() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;
    app.handle_view_manager_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));

    for expected in [1, 2, 3, 0] {
        app.handle_view_manager_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.view_form.as_ref().unwrap().focus, expected);
    }

    app.handle_view_manager_key(key_event(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(app.view_form.as_ref().unwrap().focus, 3);
}

#[test]
fn test_view_form_cancel_discards() {
    let tasks = sample_tasks();
    let views = vec![View::new("View1", "not done", "", "")];
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
    for c in "junk".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Esc, KeyModifiers::NONE));

    assert!(app.view_form.is_none());
    assert_eq!(app.views[0].name, "View1"); // unchanged
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix-shell --run "cargo test view_form"`
Expected: FAIL — compile error `no field view_form on type App`

- [ ] **Step 3: Implement ViewForm**

In `src/ui/mod.rs`:

1. Replace the App struct fields:

```rust
    pub view_edit: Option<TextArea<'static>>,
```

with:

```rust
    pub view_form: Option<view_manager::ViewForm>,
```

2. Remove `pub editing_view_field: ViewEditField,` from the struct, remove the entire `ViewEditField` enum, and remove `editing_view_field: ViewEditField::Name,` from `App::new`. Add `view_form: None,` to `App::new` in place of `view_edit: None,`.

3. In `App::draw`, replace:

```rust
        if self.show_view_manager || self.view_edit.is_some() {
            view_manager::draw(frame, self);
        }
```

with:

```rust
        if self.show_view_manager || self.view_form.is_some() {
            view_manager::draw(frame, self);
        }
```

In `src/ui/view_manager.rs`, replace the whole file content with:

```rust
use crate::ui::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use tui_textarea::{CursorMove, TextArea};

pub const FIELD_NAME: usize = 0;
pub const FIELD_QUERY: usize = 1;
pub const FIELD_SORT_BY: usize = 2;
pub const FIELD_GROUP_BY: usize = 3;
const FIELD_COUNT: usize = 4;

pub struct ViewForm {
    pub fields: [TextArea<'static>; FIELD_COUNT],
    pub focus: usize,
    pub editing_index: Option<usize>, // None = adding a new view
}

fn new_field(text: &str) -> TextArea<'static> {
    let lines = if text.is_empty() {
        vec![String::new()]
    } else {
        text.lines().map(|l| l.to_string()).collect()
    };
    let mut textarea = TextArea::new(lines);
    textarea.set_cursor_line_style(Style::default());
    textarea.move_cursor(CursorMove::End);
    textarea
}

impl ViewForm {
    fn empty() -> Self {
        let mut form = Self {
            fields: [new_field(""), new_field(""), new_field(""), new_field("")],
            focus: 0,
            editing_index: None,
        };
        form.update_focus_styles();
        form
    }

    fn for_view(idx: usize, view: &crate::view::View) -> Self {
        let mut form = Self {
            fields: [
                new_field(&view.name),
                new_field(&view.query),
                new_field(&view.sort_by),
                new_field(&view.group_by),
            ],
            focus: 0,
            editing_index: Some(idx),
        };
        form.update_focus_styles();
        form
    }

    fn update_focus_styles(&mut self) {
        for (i, field) in self.fields.iter_mut().enumerate() {
            if i == self.focus {
                field.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            } else {
                field.set_cursor_style(Style::default());
            }
        }
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.view_form.is_some() {
        handle_form_key(app, key);
        return;
    }

    let code = key.code;
    match code {
        KeyCode::Esc => {
            app.show_view_manager = false;
        }
        KeyCode::Char('j') | KeyCode::Down
            if !app.views.is_empty() =>
        {
            let i = app.view_manager_state.selected().unwrap_or(0);
            let next = if i >= app.views.len() - 1 { 0 } else { i + 1 };
            app.view_manager_state.select(Some(next));
        }
        KeyCode::Char('k') | KeyCode::Up
            if !app.views.is_empty() =>
        {
            let i = app.view_manager_state.selected().unwrap_or(0);
            let prev = if i == 0 { app.views.len() - 1 } else { i - 1 };
            app.view_manager_state.select(Some(prev));
        }
        KeyCode::Char('a') => {
            app.view_form = Some(ViewForm::empty());
        }
        KeyCode::Char('e') => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    app.view_form = Some(ViewForm::for_view(idx, view));
                }
            }
        }
        KeyCode::Char('d')
            if app.views.len() > 1 =>
        {
            if let Some(idx) = app.view_manager_state.selected() {
                if idx < app.views.len() {
                    app.views.remove(idx);
                    let new_sel = if idx >= app.views.len() { app.views.len() - 1 } else { idx };
                    app.view_manager_state.select(Some(new_sel));
                    app.save_config();
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    app.config.defaults.view = view.name.clone();
                    app.save_config();
                }
            }
        }
        KeyCode::Enter => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    app.current_view = view.clone();
                    app.show_view_manager = false;
                    app.filter_text.clear();
                    app.dirty = true;
                }
            }
        }
        _ => {}
    }
}

fn handle_form_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.view_form = None;
        }
        KeyCode::Tab => {
            if let Some(form) = &mut app.view_form {
                form.focus = (form.focus + 1) % FIELD_COUNT;
                form.update_focus_styles();
            }
        }
        KeyCode::BackTab => {
            if let Some(form) = &mut app.view_form {
                form.focus = (form.focus + FIELD_COUNT - 1) % FIELD_COUNT;
                form.update_focus_styles();
            }
        }
        KeyCode::Enter if !key.modifiers.contains(KeyModifiers::ALT) => {
            save_form(app);
        }
        _ => {
            // Includes Alt+Enter, which tui-textarea turns into a newline
            // (multi-line queries).
            if let Some(form) = &mut app.view_form {
                form.fields[form.focus].input(key);
            }
        }
    }
}

fn save_form(app: &mut App) {
    // Implemented in Task 8.
    let _ = app;
    todo!("save_form");
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    if app.view_form.is_some() {
        draw_form(frame, app);
    } else if app.show_view_manager {
        draw_list(frame, app);
    }
}

fn draw_list(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    let popup_width = 60.min(area.width - 4);
    let popup_height = (app.views.len() as u16 + 5).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

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

    let help_line = Line::from(Span::styled(
        "Enter: switch | a: add | e: edit | d: del | s: set default | Esc: close",
        Style::default().fg(Color::Gray),
    ));

    let help_item = ListItem::new(help_line);
    let mut all_items = items;
    all_items.push(help_item);

    let list = List::new(all_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Manage Views")
            .style(Style::default().bg(Color::DarkGray)),
    );

    let mut state = ListState::default();
    state.select(app.view_manager_state.selected());
    frame.render_stateful_widget(list, popup_area, &mut state);
}

fn draw_form(frame: &mut ratatui::Frame, app: &App) {
    let Some(form) = &app.view_form else { return };
    let area = frame.area();
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 13.min(area.height.saturating_sub(4));
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let title = if form.editing_index.is_some() { "Edit View" } else { "Add View" };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, popup_area);

    let inner = popup_area.inner(Margin { horizontal: 1, vertical: 1 });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // name row
            Constraint::Length(1), // query label
            Constraint::Length(3), // query textarea
            Constraint::Length(1), // sort_by row
            Constraint::Length(1), // group_by row
            Constraint::Length(1), // blank
            Constraint::Length(1), // help
        ])
        .split(inner);

    draw_inline_field(frame, &form.fields[FIELD_NAME], "Name", form.focus == FIELD_NAME, chunks[0]);

    frame.render_widget(field_label("Query", form.focus == FIELD_QUERY), chunks[1]);
    frame.render_widget(&form.fields[FIELD_QUERY], chunks[2]);

    draw_inline_field(frame, &form.fields[FIELD_SORT_BY], "Sort by", form.focus == FIELD_SORT_BY, chunks[3]);
    draw_inline_field(frame, &form.fields[FIELD_GROUP_BY], "Group by", form.focus == FIELD_GROUP_BY, chunks[4]);

    let help = Paragraph::new(Span::styled(
        "Tab: next field | Enter: save | Esc: cancel",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(help, chunks[6]);
}

fn field_label(label: &str, focused: bool) -> Paragraph<'static> {
    let style = if focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    Paragraph::new(format!("{}:", label)).style(style)
}

fn draw_inline_field(frame: &mut ratatui::Frame, textarea: &TextArea, label: &str, focused: bool, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(10), Constraint::Min(1)])
        .split(area);
    frame.render_widget(field_label(label, focused), chunks[0]);
    frame.render_widget(textarea, chunks[1]);
}
```

Note: `save_form` is a `todo!()` stub for this task; no test exercises Enter-in-form yet. The old tests `test_view_manager_navigation`, `test_view_manager_select`, `test_view_manager_delete`, `test_view_manager_esc`, `test_view_manager_set_default` must still pass unchanged.

- [ ] **Step 4: Run tests to verify**

Run: `nix-shell --run "cargo test view"`
Expected: PASS — 4 new form tests + all existing view manager tests

Run: `nix-shell --run "cargo clippy -- -D warnings"`
Expected: PASS (clippy may flag the `todo!()` stub's `let _ = app;` — if it does, drop that line; `todo!()` alone is fine)

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs src/ui/view_manager.rs
git commit -m "feat: add view edit form with field focus navigation"
```

---

### Task 8: ViewForm save — add/edit, validation, current-view sync

**Files:**
- Modify: `src/ui/view_manager.rs` (`save_form`)
- Modify: `src/ui/mod.rs` (tests)

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/ui/mod.rs`:

```rust
#[test]
fn test_view_form_add_creates_view() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
    for c in "Work".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Tab, KeyModifiers::NONE)); // to query field
    for c in "not done".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.view_form.is_none());
    assert_eq!(app.views.len(), 2);
    assert_eq!(app.views[1].name, "Work");
    assert_eq!(app.views[1].query, "not done");
}

#[test]
fn test_view_form_edit_updates_view() {
    let tasks = sample_tasks();
    let views = vec![View::new("View1", "not done", "", "")];
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
    app.handle_view_manager_key(key_event(KeyCode::Tab, KeyModifiers::NONE)); // query field, cursor at end
    for c in " tag work".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.view_form.is_none());
    assert_eq!(app.views[0].query, "not done tag work");
}

#[test]
fn test_view_form_empty_name_rejected() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.view_form.is_some()); // still open
    assert_eq!(app.status_message.as_deref(), Some("View name required"));
    assert_eq!(app.views.len(), 1);
}

#[test]
fn test_view_form_duplicate_name_rejected() {
    let tasks = sample_tasks();
    let views = vec![View::new("View1", "", "", "")];
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
    for c in "View1".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.view_form.is_some());
    assert_eq!(app.status_message.as_deref(), Some("View 'View1' already exists"));
    assert_eq!(app.views.len(), 1);
}

#[test]
fn test_view_form_rename_to_own_name_allowed() {
    let tasks = sample_tasks();
    let views = vec![View::new("View1", "not done", "", "")];
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    // Open edit form and save without changes — must not be treated as duplicate.
    app.handle_view_manager_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.view_form.is_none());
    assert!(app.status_message.is_none());
    assert_eq!(app.views[0].name, "View1");
}

#[test]
fn test_view_form_edit_current_view_syncs() {
    let tasks = sample_tasks();
    let views = vec![View::new("All Tasks", "", "", "")];
    let mut app = test_app(tasks, views);
    assert_eq!(app.current_view.name, "All Tasks");
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
    app.handle_view_manager_key(key_event(KeyCode::Tab, KeyModifiers::NONE)); // query field
    for c in "not done".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.current_view.query, "not done");
}

#[test]
fn test_view_form_query_multiline() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.show_view_manager = true;

    app.handle_view_manager_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
    for c in "V".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Tab, KeyModifiers::NONE)); // query field
    for c in "not done".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::ALT)); // newline
    for c in "tag work".chars() {
        app.handle_view_manager_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE)); // save

    assert_eq!(app.views[1].query, "not done\ntag work");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix-shell --run "cargo test view_form"`
Expected: FAIL — `test_view_form_add_creates_view` (and others) panic on `todo!("save_form")`; note the earlier Task 7 tests (open/focus/cancel) still pass

- [ ] **Step 3: Implement `save_form`**

In `src/ui/view_manager.rs`, replace the `save_form` stub with:

```rust
fn save_form(app: &mut App) {
    let Some(form) = &app.view_form else { return };
    let name = form.fields[FIELD_NAME].lines()[0].trim().to_string();
    let editing_index = form.editing_index;
    let query = form.fields[FIELD_QUERY].lines().join("\n");
    let sort_by = form.fields[FIELD_SORT_BY].lines()[0].clone();
    let group_by = form.fields[FIELD_GROUP_BY].lines()[0].clone();

    if name.is_empty() {
        app.status_message = Some("View name required".to_string());
        return;
    }

    let duplicate = app
        .views
        .iter()
        .enumerate()
        .any(|(i, v)| v.name == name && Some(i) != editing_index);
    if duplicate {
        app.status_message = Some(format!("View '{}' already exists", name));
        return;
    }

    match editing_index {
        Some(idx) => {
            let was_current = app.views[idx].name == app.current_view.name;
            {
                let view = &mut app.views[idx];
                view.name = name;
                view.query = query;
                view.sort_by = sort_by;
                view.group_by = group_by;
            }
            if was_current {
                app.current_view = app.views[idx].clone();
                app.dirty = true;
            }
        }
        None => {
            app.views
                .push(crate::view::View::new(&name, &query, &sort_by, &group_by));
        }
    }

    app.save_config();
    app.view_form = None;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix-shell --run "cargo test"`
Expected: PASS — full suite green, including all 11 view_form tests

Run: `nix-shell --run "cargo clippy -- -D warnings"`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs src/ui/view_manager.rs
git commit -m "feat: save views from form with name validation and current-view sync"
```

---

### Task 9: Docs + full verification

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: Update AGENTS.md project structure**

In `AGENTS.md`, replace in the structure listing:

```
  ui/
    mod.rs         # UI root, layout
    task_list.rs   # Main task list widget
    command.rs     # Command palette / search input
    modal.rs       # Modal editor for task editing
```

with:

```
  ui/
    mod.rs          # UI root, layout
    task_list.rs    # Main task list widget
    filter.rs       # In-view text filter (bottom bar, '/' key)
    view_manager.rs # View list + add/edit form ('v' key)
    modal.rs        # Modal editor for task editing
```

- [ ] **Step 2: Full verification**

Run: `nix-shell --run "cargo fmt --check"`
Expected: PASS (if it fails, run `nix-shell --run "cargo fmt"` and re-check)

Run: `nix-shell --run "cargo clippy -- -D warnings"`
Expected: PASS

Run: `nix-shell --run "cargo test"`
Expected: PASS — full suite green

- [ ] **Step 3: Manual smoke test (if a vault is configured)**

Run: `nix-shell --run "cargo run"` — verify:
- `/` opens bottom-bar filter, typing narrows live, Enter keeps, Esc clears
- Status bar shows `filter: "..."` while active; Esc in list clears
- `v` opens view manager; `a` adds, `e` edits all four fields, Tab moves focus, duplicate name rejected, view switch clears filter
- `?` help overlay shows the new keybindings

- [ ] **Step 4: Commit**

```bash
git add AGENTS.md
git commit -m "docs: update AGENTS.md structure for filter and view_manager modules"
```

---

## Self-Review Notes

- **Spec coverage:** filter matching (T1), pipeline (T2), keybindings/lifecycle (T3), bottom bar + indicator (T4), palette removal + help overlay (T5), view manager consolidation with `a`/`e`/form (T6–T8), form save rules incl. empty/duplicate/current-view sync/multiline query (T8), AGENTS.md + verification (T9). Out-of-scope items unchanged.
- **Type consistency:** `ViewForm { fields: [TextArea<'static>; 4], focus: usize, editing_index: Option<usize> }` used identically in T7 tests, T7 impl, T8 tests, T8 impl. `matches_filter(&Task, &str) -> bool` same everywhere. `filter::handle_key`/`filter::draw` signatures match T3/T4 usage. `view_manager::handle_key`/`view_manager::draw` signatures match the T6 wrapper.
- **Ordering:** T3 deletes `start_search` (else clippy dead-code fails after rebinding `/`); command.rs fully removed in T5; `ViewEditField`/`view_edit` removed in T7 (they are still moved in T6 to keep T6 a pure refactor).
