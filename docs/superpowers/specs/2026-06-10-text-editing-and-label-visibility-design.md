# Text Editing & Label Visibility Design

## Problem

1. **No cursor movement in text fields**: Three of four text editing contexts (edit task modal, view name editing, save-view name) only support append and backspace — no arrow keys, Home/End, or insert-at-cursor. The search/query editor has full cursor support but uses 150+ lines of manual cursor logic.

2. **Invisible labels**: In the edit task modal, field labels use `Color::DarkGray` foreground on a `Color::DarkGray` background, making them invisible.

## Solution

### 1. Add `tui-textarea` dependency

Add `tui-textarea = "0.6"` to `Cargo.toml`. v0.6.x is the version compatible with ratatui 0.28 (v0.7.0 bumped to ratatui 0.29). Supports both single-line and multi-line text editing with full cursor movement, undo/redo, and crossterm integration. Import as `tui_textarea::TextArea`.

### 2. Label color fix

Change `Color::DarkGray` → `Color::Gray` for all label/help/hint **text** (not backgrounds) across:

- `src/ui/modal.rs:286` — field label style
- `src/ui/modal.rs:233` — source file path
- `src/ui/modal.rs:246-270` — all help text lines
- `src/ui/modal.rs:303` — "e to edit" hint
- `src/ui/command.rs:233,237,301,311,315` — search/editor help text
- `src/ui/mod.rs:435` — view edit field label
- `src/ui/mod.rs:474,476` — view manager help text

Popup backgrounds (`bg(Color::DarkGray)`) stay as-is.

### 3. Migrate all editors to `TextArea`

#### App struct changes (`src/ui/mod.rs`)

Remove:
- `search_query: String`, `search_cursor_row: usize`, `search_cursor_col: usize`
- `editing_view: bool`, `editing_view_text: String`
- `saving_view: bool`, `view_name_input: String`
- `editing_task_field: bool`, `task_edit_text: String`

Add:
- `search_textarea: Option<TextArea<'static>>` — multi-line, for query editing
- `view_edit: Option<TextArea<'static>>` — single-line, for view name editing
- `save_view_edit: Option<TextArea<'static>>` — single-line, for save-view name
- `task_edit: Option<TextArea<'static>>` — single-line, for modal field editing

#### Edit task modal (`src/ui/modal.rs`)

- `start_field_edit()`: create `TextArea` with field value
- `handle_field_edit()`: intercept Enter (save) and Esc (cancel) before passing to `textarea.input(key)`. This prevents TextArea from inserting newlines, effectively creating single-line behavior.
- `draw()`: use `textarea.widget()` for the editing line instead of manual cursor rendering
- `apply_field_edit()`: read text from `textarea.lines()[0]`

#### Search/query editor (`src/ui/command.rs`)

- `start_search()` → create `TextArea` with current query, multi-line mode
- Remove `handle_key()` — intercept Alt+Enter (submit) and Esc (cancel) before passing to `textarea.input(key)`. Enter passes through to TextArea for newline insertion.
- Remove `query_lines()`, `query_line_refs()`, all manual cursor logic
- `draw()`: use `textarea.widget()`, add help text below

#### View name editing (`src/ui/mod.rs`)

- `handle_view_manager_key()` when editing: intercept Enter (save) / Esc (cancel) before passing to `textarea.input(key)`
- `draw_view_manager()`: use `textarea.widget()` for editing line

#### Save view name (`src/ui/command.rs`)

- `handle_save_view_key()`: intercept Enter (save) / Esc (cancel) before passing to `textarea.input(key)`
- `draw_save_view()`: use `textarea.widget()` for input line

### 4. Test updates

Existing tests use `KeyCode::Char` and `KeyCode::Backspace` — these still work with `TextArea::input()`. Add tests for:
- Arrow key insertion (insert "hello", move cursor left twice, insert "x" → "helxlo")
- Home/End behavior
- Search editor Alt+Enter submission

## Files changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add `tui-textarea = "0.6"` |
| `src/ui/mod.rs` | Replace App fields, update view manager editing/drawing |
| `src/ui/modal.rs` | Replace manual editing with TextArea, fix label colors |
| `src/ui/command.rs` | Replace manual editing with TextArea for both search and save-view, fix label colors |

## Scope

This is a focused UI improvement. No changes to task parsing, query language, storage, or config.
