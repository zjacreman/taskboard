# In-View Text Filter & View Manager Consolidation — Design

Date: 2026-07-17

## Overview

Two related changes:

1. **`/` becomes a transient in-view text filter.** Typing a string narrows the current
   view to tasks whose description or tags contain it, live as you type. The filter is
   never persisted to views or config.
2. **View add/edit consolidates into the view management modal.** The current `/`
   behavior (ad-hoc query editor with Ctrl+S save-as-view) is removed entirely. The view
   manager (`v`) gains `a` (add view) and a full edit form (name, query, sort_by,
   group_by) on `e`.

## Decisions (from brainstorming)

- Filter matches **description + tags**, case-insensitive substring.
- Standalone query editor is **removed entirely**; query editing lives only in the view
  manager form.
- Filter applies **live**; `Enter` keeps it and closes the input; `Esc` clears it.
- Filter **resets on view switch**.
- Filter input is a **bottom bar**, vim-style (reuses the existing status-bar line).

## Architecture

### Filter state (App)

- `filter_text: String` — active filter; empty string = no filter.
- `filter_textarea: Option<TextArea<'static>>` — `Some` while the filter input is open.

New module `src/ui/filter.rs` holds the key handler and draw code for the filter bar,
following the existing per-module pattern (`command.rs` is deleted; `modal.rs` is the
style reference).

### Filter matching

`update_filtered_tasks()` pipeline becomes:

1. Run current view query via `task::query::execute_query` (unchanged).
2. Sort by source path, then line number (unchanged).
3. If `filter_text.trim()` is non-empty, keep only tasks where:
   - `description.to_lowercase().contains(needle)`, OR
   - any tag `t` where `t.to_lowercase().contains(needle_tags)`

   where `needle = filter_text.trim().to_lowercase()` and `needle_tags` is `needle` with
   any leading `#` stripped (so both `work` and `#work` match a `#work` tag).

Existing selected-index clamping is unchanged.

### Filter keybindings

| Context | Key | Behavior |
|---|---|---|
| Main list | `/` | Open filter input, pre-filled with current `filter_text` |
| Filter input | (typing) | Update `filter_text` live, `dirty = true` |
| Filter input | `Enter` | Close input, keep filter |
| Filter input | `Esc` | Clear filter, close input |
| Main list, filter active | `Esc` | Clear filter |
| Main list, no filter | `Esc` | No-op (unchanged) |

Whitespace-only filter text is treated as no filter.

### Filter UI (bottom bar)

The existing 1-line status bar area is reused:

- **Input open:** horizontal split — a 2-column `"/ "` label plus a single-line textarea.
- **Input closed, filter active:** status bar appends `| filter: "<text>"` to the normal
  counts/view display.
- **Input closed, no filter:** unchanged.

The status-bar hint text changes from `/:search` to `/:filter`.

## View manager redesign

### State

Remove `view_edit: Option<TextArea>`, `editing_view_field: ViewEditField`, and the
`ViewEditField` enum. Add:

- `view_form: Option<ViewForm>`

```rust
pub struct ViewForm {
    pub fields: [TextArea<'static>; 4], // name, query, sort_by, group_by
    pub focus: usize,                   // 0..=3
    pub editing_index: Option<usize>,   // None = adding new view
}
```

### List-mode keys (unchanged unless noted)

| Key | Behavior |
|---|---|
| `a` | Open empty form (`editing_index = None`) |
| `e` | Open form populated from selected view (`editing_index = Some(idx)`) |
| `d` | Delete selected view (unchanged) |
| `s` | Set selected view as default (unchanged) |
| `Enter` | Switch to selected view **and clear the text filter** |
| `Esc` | Close view manager (unchanged) |

### Form-mode keys

| Key | Behavior |
|---|---|
| `Tab` / `Shift+Tab` | Move focus between the 4 fields |
| `Enter` | Save |
| `Esc` | Cancel, discard changes |
| `Alt+Enter` (query field) | Insert newline (multi-line queries, as today) |
| all other keys | Input to focused field |

### Save rules

- Empty/whitespace-only name → status message "View name required", stay in form.
- Name duplicates another existing view (excluding the view being edited) → status
  message `View '<name>' already exists`, stay in form. No silent overwrite.
- Add: push new `View { name, query, sort_by, group_by }`, `save_config()`, close form.
- Edit: write fields into `views[idx]`, `save_config()`, close form. If the edited view
  is the current view (name matched `current_view.name` before the edit), sync
  `current_view` to the edited values and set `dirty` — avoids a stale running query
  after editing the active view (and tracks renames).

### Form rendering

Centered popup in the existing view-manager style (`Clear` + bordered block, dark gray).
Four labeled fields stacked vertically: Name (1 line), Query (3 lines), Sort by (1
line), Group by (1 line). The focused field's label is highlighted and its textarea
shows the cursor. Help line: `Tab: next field | Enter: save | Esc: cancel`.

`sort_by` and `group_by` remain free text; the query engine does not consume them today.

## Removals

- Delete `src/ui/command.rs` entirely (search palette, save-view flow, overwrite
  confirm).
- Remove App state: `search_textarea`, `save_view_edit`, `save_view_confirm_overwrite`.
- Remove `start_search()`; `/` now opens the filter.
- Remove `ViewEditField` enum.
- `update_filtered_tasks()`: drop the `search_textarea` branch; always use
  `current_view.query`, then apply the text-filter pass.
- Remove the `test_search_alt_enter_submit` test (replaced by filter tests).

## Help & docs

Help overlay updates:

- `/` — "Filter tasks (text)"
- `v` — "Manage views (a:add e:edit d:del s:default)"
- Remove the stale `V` line (no such binding exists).

## Error handling

- Non-fatal conditions (empty name, duplicate name) surface via the existing
  `status_message` mechanism; the form stays open.
- `save_config()` failures are logged, as today.

## Testing

Unit tests (in `src/ui/mod.rs` / `src/ui/filter.rs` test modules):

- Filter matching: description hit, tag hit, case-insensitivity, `#`-prefix matching,
  no-match narrows to empty, whitespace-only filter is a no-op.
- Filter lifecycle: Enter keeps filter after close; Esc clears while typing; Esc in main
  list clears active filter; reopening pre-fills current filter; switching views clears
  the filter; clearing text + Enter removes the filter.
- View form: add creates a view and saves config; edit updates fields; focus navigation
  wraps both directions; empty name rejected; duplicate name rejected (but renaming to
  the view's own current name is allowed); editing the current view syncs
  `current_view`; cancel discards changes.

Existing view-manager tests (navigation, delete, select, esc) remain valid; tests
referencing removed state (`search_textarea`) are updated or removed.

## Out of scope

- Regex/fuzzy matching, negation, or multi-term filter syntax.
- `sort_by`/`group_by` becoming functional in the query engine.
- Changing how `current_view` relates to `views` beyond the edit-sync rule above
  (e.g., deleting the current view keeps current behavior).
