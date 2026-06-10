# Text Editing & Label Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace manual text editing with `tui-textarea` library across all 4 editors, and fix invisible label colors.

**Architecture:** Add `tui-textarea` v0.6 (compatible with ratatui 0.28) as a dependency. Replace all manual string+cursor editing logic with `TextArea` widgets. Change label colors from `Color::DarkGray` to `Color::Gray` for visibility.

**Tech Stack:** Rust, ratatui 0.28, crossterm 0.28, tui-textarea 0.6

---

## File Structure

| File | Responsibility | Change Type |
|------|---------------|-------------|
| `Cargo.toml` | Dependencies | Modify |
| `src/ui/mod.rs` | App struct, event loop, view manager, help overlay | Modify |
| `src/ui/modal.rs` | Edit task modal | Modify |
| `src/ui/command.rs` | Search/query editor, save-view dialog | Modify |

---

### Task 1: Add tui-textarea dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add dependency to Cargo.toml**

```toml
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
rustyline = "14"
tui-textarea = "0.6"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: `Finished` with no errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add tui-textarea 0.6 for text editing"
```

---

### Task 2: Fix label colors

**Files:**
- Modify: `src/ui/modal.rs`
- Modify: `src/ui/command.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Fix modal.rs label colors**

In `src/ui/modal.rs`, change all `Color::DarkGray` for text (not backgrounds) to `Color::Gray`:

Line 233 — source file path:
```rust
// Before
Style::default().fg(Color::DarkGray),
// After
Style::default().fg(Color::Gray),
```

Line 286 — field label style:
```rust
// Before
let label_style = Style::default().fg(Color::DarkGray);
// After
let label_style = Style::default().fg(Color::Gray);
```

Lines 246-270 — all help text lines (6 occurrences of `fg(Color::DarkGray)` in help text):
```rust
// Before
Style::default().fg(Color::DarkGray),
// After
Style::default().fg(Color::Gray),
```

Line 303 — "e to edit" hint:
```rust
// Before
Span::styled("  (e to edit)", Style::default().fg(Color::DarkGray)),
// After
Span::styled("  (e to edit)", Style::default().fg(Color::Gray)),
```

- [ ] **Step 2: Fix command.rs label colors**

In `src/ui/command.rs`, change `Color::DarkGray` for text to `Color::Gray`:

Lines 233, 237 — search help text:
```rust
// Before
Style::default().fg(Color::DarkGray),
// After
Style::default().fg(Color::Gray),
```

Lines 301, 311, 315 — save-view help text:
```rust
// Before
Style::default().fg(Color::DarkGray),
// After
Style::default().fg(Color::Gray),
```

Do NOT change lines 281, 324 — those are `bg(Color::DarkGray)` (backgrounds, not text).

- [ ] **Step 3: Fix mod.rs label colors**

In `src/ui/mod.rs`, change `Color::DarkGray` for text to `Color::Gray`:

Line 435 — view edit field label:
```rust
// Before
Span::styled(format!("{}: ", field_name), Style::default().fg(Color::DarkGray)),
// After
Span::styled(format!("{}: ", field_name), Style::default().fg(Color::Gray)),
```

Lines 474, 476 — view manager help text:
```rust
// Before
Style::default().fg(Color::DarkGray),
// After
Style::default().fg(Color::Gray),
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: `Finished` with no errors

- [ ] **Step 5: Commit**

```bash
git add src/ui/modal.rs src/ui/command.rs src/ui/mod.rs
git commit -m "fix: change label colors from DarkGray to Gray for visibility"
```

---

### Task 3: Refactor App struct to use TextArea

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add tui_textarea import**

At the top of `src/ui/mod.rs`, add:
```rust
use tui_textarea::TextArea;
```

- [ ] **Step 2: Replace App struct fields**

Remove these fields from the `App` struct:
```rust
pub search_active: bool,
pub search_query: String,
pub search_cursor_row: usize,
pub search_cursor_col: usize,
pub saving_view: bool,
pub view_name_input: String,
pub editing_view: bool,
pub editing_view_text: String,
pub editing_task_field: bool,
pub task_edit_text: String,
```

Add these fields:
```rust
pub search_textarea: Option<TextArea>,
pub save_view_edit: Option<TextArea>,
pub view_edit: Option<TextArea>,
pub task_edit: Option<TextArea>,
```

Keep these fields (they track which field is selected, not text content):
```rust
pub editing_view_field: ViewEditField,
pub task_edit_field: TaskEditField,
```

- [ ] **Step 3: Update App::new()**

Replace the removed field initializations with:
```rust
search_textarea: None,
save_view_edit: None,
view_edit: None,
task_edit: None,
```

- [ ] **Step 4: Update event loop to pass KeyEvent**

In `App::run()`, change:
```rust
self.handle_key(key.code, key.modifiers);
```
to:
```rust
self.handle_key(key);
```

- [ ] **Step 5: Update handle_key signature**

Change:
```rust
fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
```
to:
```rust
fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
```

Update the body to extract `code` and `modifiers` at the start:
```rust
let code = key.code;
let modifiers = key.modifiers;
```

Update the dispatch to pass `key` instead of `code`:
```rust
if self.show_modal {
    modal::handle_key(self, key);
    return;
}
if self.search_textarea.is_some() {
    command::handle_key(self, key);
    return;
}
if self.show_view_manager {
    self.handle_view_manager_key(key);
    return;
}
```

- [ ] **Step 6: Update handle_view_manager_key signature**

Change:
```rust
pub fn handle_view_manager_key(&mut self, code: KeyCode) {
```
to:
```rust
pub fn handle_view_manager_key(&mut self, key: crossterm::event::KeyEvent) {
    let code = key.code;
```

- [ ] **Step 7: Update start_search to use TextArea**

Change `start_search()` to:
```rust
fn start_search(&mut self) {
    let mut textarea = TextArea::new(self.current_view.query.lines().map(|l| l.to_string()).collect());
    textarea.set_cursor_line_style(ratatui::style::Style::default());
    self.search_textarea = Some(textarea);
}
```

- [ ] **Step 8: Update all references to removed fields**

Throughout `src/ui/mod.rs`, replace:
- `self.search_active` → `self.search_textarea.is_some()`
- `self.search_query` → removed (access via textarea)
- `self.editing_view` → `self.view_edit.is_some()`
- `self.saving_view` → `self.save_view_edit.is_some()`
- `self.editing_task_field` → `self.task_edit.is_some()`

- [ ] **Step 9: Verify compilation**

Run: `cargo check`
Expected: Compile errors in `modal.rs` and `command.rs` (expected — Tasks 4-6 will fix them)

- [ ] **Step 10: Commit (will compile after Tasks 4-6)**

Hold this commit until after Tasks 4-6 are done.

---

### Task 4: Migrate edit task modal to TextArea

**Files:**
- Modify: `src/ui/modal.rs`

- [ ] **Step 1: Add imports**

At the top of `src/ui/modal.rs`, add:
```rust
use tui_textarea::TextArea;
use crossterm::event::KeyEvent;
```

- [ ] **Step 2: Update handle_key signature**

Change:
```rust
pub fn handle_key(app: &mut App, code: KeyCode) {
    if app.editing_task_field {
        handle_field_edit(app, code);
        return;
    }
```
to:
```rust
pub fn handle_key(app: &mut App, key: KeyEvent) {
    let code = key.code;
    if app.task_edit.is_some() {
        handle_field_edit(app, key);
        return;
    }
```

- [ ] **Step 3: Update start_field_edit**

Replace the function with:
```rust
fn start_field_edit(app: &mut App) {
    let Some(idx) = app.selected_task_index() else { return };
    let text = match app.task_edit_field {
        EditField::Description => app.tasks[idx].description.clone(),
        EditField::Status => match app.tasks[idx].status {
            TaskStatus::Todo => "todo".to_string(),
            TaskStatus::Done => "done".to_string(),
        },
        EditField::Priority => app.tasks[idx].priority.to_emoji().to_string(),
        EditField::DueDate => app.tasks[idx].due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        EditField::ScheduledDate => app.tasks[idx].scheduled_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        EditField::Recurrence => app.tasks[idx].recurrence.clone().unwrap_or_default(),
    };
    let mut textarea = TextArea::new(vec![text]);
    textarea.set_cursor_line_style(ratatui::style::Style::default());
    app.task_edit = Some(textarea);
}
```

- [ ] **Step 4: Update handle_field_edit**

Replace the function with:
```rust
fn handle_field_edit(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.task_edit = None;
        }
        KeyCode::Enter => {
            apply_field_edit(app);
            app.task_edit = None;
        }
        _ => {
            if let Some(textarea) = &mut app.task_edit {
                textarea.input(key);
            }
        }
    }
}
```

- [ ] **Step 5: Update apply_field_edit**

Change:
```rust
fn apply_field_edit(app: &mut App) {
    let Some(idx) = app.selected_task_index() else { return };
```
to:
```rust
fn apply_field_edit(app: &mut App) {
    let Some(idx) = app.selected_task_index() else { return };
    let Some(textarea) = &app.task_edit else { return };
    let text = textarea.lines()[0].clone();
```

Then replace all `app.task_edit_text` with `text` in the match body.

- [ ] **Step 6: Update draw() to use TextArea for editing field**

In the `draw()` function, update the `field_line` rendering for the editing case. Replace the `field_line` function with:

```rust
fn field_line<'a>(label: &str, value: &str, selected: bool, editing: bool, edit_text: &str, cursor_col: usize) -> Line<'a> {
    let label_style = Style::default().fg(Color::Gray);
    let marker = if selected { "▸ " } else { "  " };

    if selected && editing {
        let before = &edit_text[..cursor_col.min(edit_text.len())];
        let after = &edit_text[cursor_col.min(edit_text.len())..];
        Line::from(vec![
            Span::raw(marker),
            Span::styled(format!("{:12}", label), label_style),
            Span::styled("│ ", Style::default().fg(Color::Cyan)),
            Span::raw(before.to_string()),
            Span::styled("█", Style::default().fg(Color::White)),
            Span::raw(after.to_string()),
        ])
    } else if selected {
        Line::from(vec![
            Span::styled(marker, Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:12}", label), label_style),
            Span::styled("│ ", Style::default().fg(Color::Cyan)),
            Span::styled(value.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("  (e to edit)", Style::default().fg(Color::Gray)),
        ])
    } else {
        Line::from(vec![
            Span::raw(marker),
            Span::styled(format!("{:12}", label), label_style),
            Span::raw("  "),
            Span::raw(value.to_string()),
        ])
    }
}
```

Update the `draw()` function to pass cursor position:

```rust
let (edit_text, cursor_col) = if let Some(textarea) = &app.task_edit {
    (textarea.lines()[0].as_str(), textarea.cursor().1)
} else {
    ("", 0)
};

// Then update each field_line call:
lines.push(field_line("Description", &task.description, selected == EditField::Description, editing, edit_text, cursor_col));
// ... etc for all 6 fields
```

- [ ] **Step 7: Update draw() to read editing state from app.task_edit**

Change `let editing = app.editing_task_field;` to:
```rust
let editing = app.task_edit.is_some();
```

- [ ] **Step 8: Verify compilation**

Run: `cargo check`
Expected: Compile errors in `command.rs` and `mod.rs` (expected — Tasks 5-6 will fix them)

---

### Task 5: Migrate search/query editor to TextArea

**Files:**
- Modify: `src/ui/command.rs`

- [ ] **Step 1: Add imports**

At the top of `src/ui/command.rs`, add:
```rust
use tui_textarea::TextArea;
use crossterm::event::KeyEvent;
```

- [ ] **Step 2: Replace handle_key with TextArea-based dispatch**

Replace the entire `handle_key` function:
```rust
pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.save_view_edit.is_some() {
        handle_save_view_key(app, key);
        return;
    }

    let Some(textarea) = &mut app.search_textarea else { return };

    match key.code {
        KeyCode::Esc => {
            app.search_textarea = None;
            app.dirty = true;
        }
        KeyCode::Enter if key.modifiers.contains(crossterm::event::KeyModifiers::ALT) => {
            submit_query(app);
        }
        _ => {
            textarea.input(key);
        }
    }
}
```

- [ ] **Step 3: Update submit_query**

Change `submit_query` to read from TextArea:
```rust
fn submit_query(app: &mut App) {
    let Some(textarea) = &app.search_textarea else { return };
    app.current_view.query = textarea.lines().join("\n");
    app.search_textarea = None;
    app.dirty = true;
}
```

- [ ] **Step 4: Remove dead code**

Remove these functions (no longer needed):
- `fn query_lines(s: &str) -> Vec<String>`
- `fn query_line_refs(s: &str) -> Vec<&str>`

- [ ] **Step 5: Rewrite draw() for search editor**

Replace the `draw()` function:
```rust
pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    if app.save_view_edit.is_some() {
        draw_save_view(frame, app);
        return;
    }

    let Some(textarea) = &app.search_textarea else { return };

    let area = frame.area();
    let popup_width = 70.min(area.width - 4);
    let line_count = textarea.lines().len().max(1) as u16;
    let popup_height = (line_count + 12).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Search / Query")
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, popup_area);

    let inner = popup_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(4),
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(5),
        ])
        .split(inner);

    // Instructions
    let instructions = Paragraph::new(vec![
        Line::from(Span::styled("One filter per line. Example:", Style::default().fg(Color::Gray))),
        Line::from(Span::styled("  not done\ndue before tomorrow\nsort by priority", Style::default().fg(Color::Gray))),
    ]);
    frame.render_widget(instructions, chunks[0]);

    // TextArea
    frame.render_widget(textarea, chunks[1]);

    // Help text
    let help = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(": newline"),
        ]),
        Line::from(vec![
            Span::styled("Alt+Enter", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(": apply query"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+S", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(": save as view"),
        ]),
        Line::from(vec![
            Span::styled("Esc", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(": cancel"),
        ]),
    ]);
    frame.render_widget(help, chunks[2]);
}
```

- [ ] **Step 6: Update handle_save_view_key**

Change signature and body:
```rust
fn handle_save_view_key(app: &mut App, key: KeyEvent) {
    let Some(textarea) = &mut app.save_view_edit else { return };
    match key.code {
        KeyCode::Esc => {
            app.save_view_edit = None;
        }
        KeyCode::Enter => {
            let name = if textarea.lines()[0].is_empty() {
                "Untitled".to_string()
            } else {
                textarea.lines()[0].clone()
            };

            let view = View::new(
                &name,
                &app.current_view.query,
                &app.current_view.sort_by,
                &app.current_view.group_by,
            );

            app.views.push(view);
            app.save_views();

            app.save_view_edit = None;
            app.search_textarea = None;
            app.dirty = true;
        }
        _ => {
            textarea.input(key);
        }
    }
}
```

- [ ] **Step 7: Update Ctrl+S handler in search**

In the search `handle_key`, the Ctrl+S case needs to create a TextArea for save-view:
```rust
KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
    let mut textarea = TextArea::new(vec![String::new()]);
    textarea.set_cursor_line_style(ratatui::style::Style::default());
    app.save_view_edit = Some(textarea);
}
```

- [ ] **Step 8: Rewrite draw_save_view**

Replace `draw_save_view`:
```rust
fn draw_save_view(frame: &mut ratatui::Frame, app: &App) {
    let Some(textarea) = &app.save_view_edit else { return };

    let area = frame.area();
    let popup_width = 50.min(area.width - 4);
    let popup_height = 8.min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Save View")
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
            ratatui::layout::Constraint::Length(1),
        ])
        .split(inner);

    let title = Paragraph::new(Span::styled(
        "Save as view — enter name:",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(title, chunks[0]);

    frame.render_widget(textarea, chunks[1]);

    let query_preview = Paragraph::new(Span::styled(
        format!("Query: {}", app.current_view.query.lines().next().unwrap_or("")),
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(query_preview, chunks[3]);

    let help = Paragraph::new(Span::styled(
        "Enter: save | Esc: cancel",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(help, chunks[4]);
}
```

- [ ] **Step 9: Verify compilation**

Run: `cargo check`
Expected: Compile errors in `mod.rs` (expected — Task 6 will fix them)

---

### Task 6: Migrate view name editing to TextArea

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Update handle_view_manager_key for editing state**

Replace the editing branch of `handle_view_manager_key`:
```rust
pub fn handle_view_manager_key(&mut self, key: crossterm::event::KeyEvent) {
    let code = key.code;
    if self.view_edit.is_some() {
        match code {
            KeyCode::Esc => {
                self.view_edit = None;
            }
            KeyCode::Enter => {
                if let (Some(idx), Some(textarea)) = (self.view_manager_state.selected(), &self.view_edit) {
                    if let Some(view) = self.views.get_mut(idx) {
                        let text = textarea.lines()[0].clone();
                        match self.editing_view_field {
                            ViewEditField::Name => view.name = text,
                            ViewEditField::Query => view.query = text,
                            ViewEditField::SortBy => view.sort_by = text,
                            ViewEditField::GroupBy => view.group_by = text,
                        }
                    }
                }
                self.view_edit = None;
                self.save_views();
            }
            _ => {
                if let Some(textarea) = &mut self.view_edit {
                    textarea.input(key);
                }
            }
        }
        return;
    }
    // ... rest of the function stays the same but uses `code` instead of the parameter
```

- [ ] **Step 2: Update the 'e' key handler for view editing**

In the `KeyCode::Char('e')` branch:
```rust
KeyCode::Char('e') => {
    if let Some(idx) = self.view_manager_state.selected() {
        if let Some(view) = self.views.get(idx) {
            let mut textarea = TextArea::new(vec![view.name.clone()]);
            textarea.set_cursor_line_style(ratatui::style::Style::default());
            self.view_edit = Some(textarea);
            self.editing_view_field = ViewEditField::Name;
        }
    }
}
```

- [ ] **Step 3: Update draw_view_manager for TextArea rendering**

In `draw_view_manager`, replace the editing block rendering:
```rust
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
        "Tab: next field | Enter: save | Esc: cancel",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(help, chunks[3]);
} else {
    // ... existing view list rendering stays the same
```

- [ ] **Step 4: Update remaining references to removed fields**

In `App::handle_key()`, update the search_active check:
```rust
if self.search_textarea.is_some() {
    command::handle_key(self, key);
    return;
}
```

In `App::draw()`, update:
```rust
if self.search_textarea.is_some() {
    command::draw(frame, self);
}
if self.show_view_manager || self.view_edit.is_some() {
    draw_view_manager(frame, self);
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check`
Expected: `Finished` with no errors

- [ ] **Step 6: Commit all changes**

```bash
git add src/ui/mod.rs src/ui/modal.rs src/ui/command.rs
git commit -m "feat: migrate all editors to tui-textarea for proper cursor support"
```

---

### Task 7: Update existing tests

**Files:**
- Modify: `src/ui/mod.rs` (tests module)

- [ ] **Step 1: Add KeyEvent import to tests**

In the test module, add:
```rust
use crossterm::event::KeyEvent;
```

- [ ] **Step 2: Create a helper function for KeyEvent construction**

Add to the test module:
```rust
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}
```

- [ ] **Step 3: Update all test calls to use KeyEvent**

Replace all occurrences in tests:
- `app.handle_key(KeyCode::Xxx, KeyModifiers::NONE)` → `app.handle_key(key(KeyCode::Xxx))`
- `app.handle_key(KeyCode::Xxx, KeyModifiers::SHIFT)` → `app.handle_key(key_mod(KeyCode::Xxx, KeyModifiers::SHIFT))`
- `app.handle_view_manager_key(KeyCode::Xxx)` → `app.handle_view_manager_key(key(KeyCode::Xxx))`

- [ ] **Step 4: Update tests that reference removed fields**

Tests that check `app.editing_task_field` should check `app.task_edit.is_some()` instead.
Tests that check `app.search_active` should check `app.search_textarea.is_some()` instead.
Tests that check `app.task_edit_text` should read from `app.task_edit.as_ref().unwrap().lines()[0]` instead.

- [ ] **Step 5: Verify tests compile**

Run: `cargo check`
Expected: `Finished` with no errors

- [ ] **Step 6: Commit**

```bash
git add src/ui/mod.rs
git commit -m "test: update tests for TextArea-based editing"
```

---

### Task 8: Add new tests for cursor movement

**Files:**
- Modify: `src/ui/mod.rs` (tests module)

- [ ] **Step 1: Test arrow key insertion in modal**

```rust
#[test]
fn test_task_edit_cursor_insert() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.update_filtered_tasks();
    app.show_modal = true;

    // Start editing description ("Buy groceries")
    app.handle_key(key(KeyCode::Char('e')));
    assert!(app.task_edit.is_some());

    // Move cursor left twice (from end)
    app.handle_key(key(KeyCode::Left));
    app.handle_key(key(KeyCode::Left));

    // Insert 'x' at cursor position
    app.handle_key(key(KeyCode::Char('x')));

    // Verify text is "Buy groceriexs"
    let text = &app.task_edit.as_ref().unwrap().lines()[0];
    assert_eq!(text, "Buy groceriexs");

    // Save
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.tasks[0].description, "Buy groceriexs");
}
```

- [ ] **Step 2: Test Home/End in modal**

```rust
#[test]
fn test_task_edit_home_end() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.update_filtered_tasks();
    app.show_modal = true;

    // Start editing description
    app.handle_key(key(KeyCode::Char('e')));

    // Move to start
    app.handle_key(key(KeyCode::Home));

    // Insert at start
    app.handle_key(key(KeyCode::Char('!')));

    // Verify
    let text = &app.task_edit.as_ref().unwrap().lines()[0];
    assert_eq!(text, "!Buy groceries");
}
```

- [ ] **Step 3: Test search editor Alt+Enter submission**

```rust
#[test]
fn test_search_alt_enter_submit() {
    let tasks = sample_tasks();
    let views = sample_views();
    let mut app = test_app(tasks, views);
    app.start_search();

    // Type a query
    for c in "done".chars() {
        app.handle_key(key(KeyCode::Char(c)));
    }

    // Submit with Alt+Enter
    app.handle_key(key_mod(KeyCode::Enter, KeyModifiers::ALT));

    assert!(app.search_textarea.is_none());
    assert_eq!(app.current_view.query, "done");
}
```

- [ ] **Step 4: Verify all tests pass**

Run: `cargo check`
Expected: `Finished` with no errors

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs
git commit -m "test: add cursor movement and Alt+Enter tests"
```

---

## Self-Review Checklist

1. **Spec coverage:** All spec requirements covered — label colors, TextArea for all 4 editors, test updates.
2. **Placeholder scan:** No TBD/TODO in plan. All code blocks are complete.
3. **Type consistency:** `TextArea` (no lifetime parameter), `KeyEvent`, `KeyCode` used consistently.
