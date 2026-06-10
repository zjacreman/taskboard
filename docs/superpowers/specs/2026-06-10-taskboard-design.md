# TaskBoard Design Spec

## Overview

TaskBoard is a TUI application for managing markdown task checkboxes from Obsidian vaults. It provides a unified interface for querying, filtering, and editing tasks scattered across multiple markdown files, using the Obsidian Tasks plugin syntax.

## Tech Stack

- **Language:** Rust
- **TUI framework:** Ratatui
- **Filesystem watching:** notify crate (inotify/FSEvents)
- **Parallel file reading:** rayon
- **Config format:** TOML
- **Development environment:** Nix (shell.nix)

## Requirements

- Configurable with an Obsidian workspace path
- Config read from `./config.toml` or `~/.config/taskboard/config.toml` (in that order)
- Find all markdown task checkboxes in the workspace
- Queryable/filterable with full Obsidian Tasks query syntax
- Save queries as reusable "views"
- Manage views (create, edit, delete, reorder)
- Edit tasks: quick keyboard shortcuts + full modal editor
- Entirely keyboard-driven
- Responsive to terminal size

## Architecture

### Approach

Hybrid: study Obsidian Tasks source for edge cases and validation, implement cleanly in Rust. Use their user guide as the spec, their test cases for validation.

### Module Structure

```
taskboard/
├── src/
│   ├── main.rs          — entry point, CLI args, app init
│   ├── config.rs        — config.toml loading/saving
│   ├── task/
│   │   ├── mod.rs       — Task struct and types
│   │   ├── parser.rs    — extract tasks from markdown files
│   │   └── query.rs     — query engine (filter/sort/group)
│   ├── vault.rs         — filesystem walker, file discovery, notify watcher
│   ├── view.rs          — saved views (query + display settings)
│   ├── ui/
│   │   ├── mod.rs       — app state, event loop
│   │   ├── task_list.rs — main task list widget
│   │   ├── modal.rs     — task editor modal
│   │   ├── command.rs   — command palette / search
│   │   └── theme.rs     — color scheme, responsive breakpoints
│   └── storage.rs       — views.toml read/write
├── tests/
│   ├── fixtures/        — sample .md files with various task formats
│   └── *.rs             — integration tests
└── shell.nix            — Nix dev environment
```

### Module Boundaries

- `task/parser.rs` — pure data extraction, no UI or disk knowledge
- `task/query.rs` — standalone engine: `fn query(tasks: &[Task], query: &str) -> Vec<Task>`
- `ui/` — all Ratatui rendering, never touches disk directly
- `vault.rs` — file discovery, watching, and reading

## Data Flow

### Task Discovery

1. **Startup:** Walk workspace directory recursively
   - Skip: `.git/`, `node_modules/`, `.obsidian/`, dotfiles
   - Filter: `*.md` files only
   - Parallel read with rayon (~200ms for 1000 files)
2. **Parsing:** Line-by-line scan for `- [ ]` or `- [x]` pattern
   - Extract: status, description, emoji metadata (📅 due, 🛫 scheduled, 🔁 recurrence, ⏫ priority)
   - Preserve: source file path, line number
3. **In-memory store:** All tasks loaded at startup
   - Query engine operates on this
   - Updated in real-time from filesystem events
   - Manual `Ctrl+r` for full rescan

### Filesystem Watching

- One recursive `notify` watcher for the whole workspace
- 100ms debounce to avoid rapid-fire re-parsing
- On file change: re-parse only that file, update in-memory store
- Events flow through `mpsc::channel` to main app loop between UI frames

### Task Editing

- Edits write back to the original markdown file at the exact line
- After writing, re-parse the modified file to update in-memory store
- Filesystem watcher also picks up the change (double update is fine, idempotent)

## Task Model

```rust
struct Task {
    description: String,
    status: TaskStatus,       // Todo, Done
    priority: Priority,       // None, Low, Medium, High (🔺⏫🔽⏬)
    due_date: Option<Date>,
    scheduled_date: Option<Date>,
    recurrence: Option<String>,
    done_date: Option<Date>,
    start_date: Option<Date>,
    tags: Vec<String>,
    source_file: PathBuf,
    line_number: usize,
}
```

Emoji mapping (Obsidian Tasks syntax):
- `📅` — due date
- `🛫` — scheduled date
- `🔁` — recurrence (e.g., "every week", "every 2 weeks", "every month on the 1st", "every weekday")
- `⏫🔼🔽⏬` — priority (high, medium, low, lowest)
- `✅` — done date
- `⏳` — start/waiting date (optional, may not be used)

## Query Engine

Full parity with Obsidian Tasks query language.

**Input:** Query string like `not done due before tomorrow sort by priority`

**Processing:**
1. Tokenize query string
2. Build filter tree (composable predicates)
3. Apply filters: `Task → bool`
4. Sort by specified fields (stable sort, multiple keys with tiebreaking)
5. Group by specified field (post-sort partition)

**Supported filters:**
- `done` / `not done`
- `includes <text>`
- `description includes <text>`
- `heading includes <text>`
- `tag <tag>` / `tags include <tag>`
- `folder <path>`
- `due before/after/on <date>`
- `scheduled before/after/on <date>`
- `happens before/after/on <date>`
- `priority is above/below/low/medium/high/none`
- `has recurrence` / `recurrence includes <text>`
- `created before/after/on <date>` (explicit ✅ emoji, falls back to file mtime)
- `limit <n>`

**Sort fields:** `due`, `scheduled`, `priority`, `description`, `tag`, `folder`, `done`, `created`

**Group fields:** `due`, `scheduled`, `priority`, `tag`, `folder`, `recurrence`

The query engine is pure — no side effects, no disk access.

## Config

### config.toml

```toml
[workspace]
path = "/home/user/obsidian-vault"

[defaults]
view = "All Tasks"

[theme]
colors = "dark"  # dark | light | custom
```

**Resolution:** Check `./config.toml` first, fall back to `~/.config/taskboard/config.toml`. If neither exists, prompt for workspace path on first run and create the file.

### views.toml

```toml
[[views]]
name = "All Tasks"
query = ""
sort_by = "due_date"
group_by = ""

[[views]]
name = "This Week"
query = "due before next sunday"
sort_by = "priority"
group_by = "due_date"
```

Stored at `~/.config/taskboard/views.toml`. Managed from within the app (add/edit/delete/reorder).

### Theme

- `dark` — built-in dark theme (default)
- `light` — built-in light theme
- `custom` — read colors from `[theme.colors]` section (future extension)

## TUI Design

### Layout

Single pane, responsive to terminal size:
- **Wide (120+ cols):** All metadata inline — checkbox, text, dates, priority, source file
- **Narrow (<80 cols):** Metadata wraps below each task, source paths hidden, dates trimmed

### Keybindings

**Navigation:**
- `j` / `k` — move up/down
- `g` / `G` — jump to top/bottom

**Quick actions (inline, no modal):**
- `x` — toggle done
- `p` — cycle priority
- `d` — set due date: today
- `D` — set due date: tomorrow
- `s` — set scheduled date: today
- `S` — set scheduled date: tomorrow
- `b` — bump scheduled date forward one day

**Views and search:**
- `/` — open inline search/query input
- `v` — switch view (dropdown)
- `V` — manage views (list: edit/delete/reorder)

**Task editing:**
- `Enter` — open modal editor for selected task

**App:**
- `Ctrl+r` — rescan vault
- `?` — help overlay
- `q` — quit

### Modal Editor

Full task editing in a modal overlay:
- Description (text input)
- Status (Tab to cycle: `[ ]` / `[x]`)
- Priority (Tab to cycle: none/🔺/⏫/🔼/🔽/⏬)
- Due date (text input, date format)
- Scheduled date (text input, date format)
- Recurrence (text input, e.g., "every week")

Navigation: `Tab` next field, `Enter` save, `Esc` cancel.

### View Management

- `v` opens dropdown of saved views, select to load
- `V` opens list of all views with options to edit, delete, or reorder
- `/` search can be saved as a new view with `V`

### Status Bar

Bottom of screen shows:
- Task count (total, done, open)
- Current view name
- Active filter (if any)
- Keybinding hints (context-sensitive)

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Config missing/invalid | Show error in TUI, prompt for workspace path, offer to create config |
| Workspace path doesn't exist | Error message on startup, exit with code 1 |
| File deleted while editing task | Show error in modal, discard changes, refresh task list |
| File modified externally | Show warning, re-parse file, highlight affected task |
| Invalid query syntax | Show parse error inline in search bar, keep previous results |
| Write permission denied | Show error, offer to save task changes to local buffer |
| Terminal too small (< 40x10) | Show "resize terminal" message with minimum dimensions |

**Principle:** Never crash on user-facing errors. Show the error in the TUI, keep the app running, offer recovery options where possible.

**Logging:** Debug logs to `~/.local/share/taskboard/taskboard.log` (XDG data dir). Not shown in TUI unless explicitly requested.

## Testing

### Unit Tests (in each module)

- `parser.rs` — task extraction from sample markdown fixtures
- `query.rs` — filter/sort/group against fixture tasks
- `config.rs` — TOML parsing, defaults, error cases
- `view.rs` — view serialization roundtrip

### Integration Tests (tests/ directory)

- End-to-end: create vault → parse → query → verify results
- File watching: modify file → verify store updates
- Task editing: edit task → verify file written correctly

### Fixtures (tests/fixtures/)

- Sample .md files with various task formats
- Edge cases: nested tasks, multi-line, malformed
- Real-world examples from Obsidian Tasks user guide

### Manual Testing

- TUI rendering at various terminal sizes
- Real Obsidian vault with thousands of files

## Dependencies

```toml
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
notify = "6"
rayon = "1"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
chrono = "0.4"
```

## Open Questions

None — all major design decisions resolved during brainstorming.
