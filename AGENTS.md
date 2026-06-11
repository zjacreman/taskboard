# TaskBoard - Agent Instructions

## Project Overview

TaskBoard is a Rust TUI application for managing Obsidian-style markdown tasks. It scans an Obsidian vault for task checkboxes, presents them in a filterable/queryable interface, and supports saving reusable views. Built with Ratatui and Crossterm.

## Development Environment

**Always use `nix-shell`** for all build, test, and lint commands. The project depends on Nix-provided toolchain:

```bash
nix-shell --run "cargo build"
nix-shell --run "cargo test"
nix-shell --run "cargo clippy -- -D warnings"
nix-shell --run "cargo fmt --check"
```

If new system packages are needed, add them to `shell.nix` (not to the system directly).

## Build & Verification Commands

| Action | Command |
|--------|---------|
| Build | `nix-shell --run "cargo build"` |
| Test | `nix-shell --run "cargo test"` |
| Lint | `nix-shell --run "cargo clippy -- -D warnings"` |
| Format check | `nix-shell --run "cargo fmt --check"` |
| Format | `nix-shell --run "cargo fmt"` |

**Always run clippy and fmt --check before claiming work is complete.**

## Project Structure

```
src/
  main.rs          # Entry point, app initialization, event loop
  lib.rs           # Library root, re-exports
  config.rs        # Config loading/saving (~/.config/taskboard/config.toml or ./config.toml)
  vault.rs         # Filesystem scanning, markdown file discovery, task extraction
  view.rs          # View management (named filtered views)
  task/
    mod.rs         # Task struct, task metadata (priority, dates, tags)
    parser.rs      # Markdown task checkbox parsing
    query.rs       # Query/filter engine (Obsidian Tasks syntax)
  ui/
    mod.rs         # UI root, layout
    task_list.rs   # Main task list widget
    command.rs     # Command palette / search input
    modal.rs       # Modal editor for task editing
  test_helpers.rs  # Shared test utilities
tests/
  integration_test.rs  # Integration tests
  fixtures/            # Test markdown files (basic.md, edge_cases.md, full_metadata.md, large.md)
docs/
  superpowers/         # Specs and implementation plans
```

## Architecture Patterns

- **Config**: TOML-based with serde. Views are stored in config as `[[views]]` arrays. Config has `save()` method for writing back to disk.
- **Task Model**: Tasks are parsed from markdown `- [ ]` / `- [x]` syntax. Metadata (due date, priority, scheduled date, tags) uses Obsidian Tasks inline field format.
- **Query Engine**: Filters tasks using Obsidian Tasks query syntax (due dates, priorities, tags, status).
- **UI**: Ratatui-based. Modal pattern for editing. Command palette for search/view switching. Keybindings are vim-inspired.
- **Vault Watching**: Uses `notify` crate for filesystem events. Supports manual rescan with Ctrl+r.
- **Parallelism**: Uses `rayon` for parallel vault scanning.

## Code Conventions

- Rust edition 2021
- Use `serde` derives for serialization
- Prefer `Result<T, E>` error handling over panics
- Keep UI code in `src/ui/`, business logic in `src/task/`
- Test fixtures go in `tests/fixtures/`
- Follow existing keybinding patterns when adding new features

## Workflow

The project uses a spec-then-plan workflow stored in `docs/superpowers/`:

1. **Specs** go in `docs/superpowers/specs/` - design documents with date prefix
2. **Plans** go in `docs/superpowers/plans/` - implementation plans with date prefix
3. File naming: `YYYY-MM-DD-short-description.md`

When working on new features, check existing specs/plans first for context.

## Configuration

Config file locations (checked in order):
1. `~/.config/taskboard/config.toml`
2. `./config.toml`

Config structure:
- `[workspace]` - vault path
- `[defaults]` - default view name
- `[theme]` - UI theme settings
- `[[views]]` - saved views with name, query, sort_by, group_by

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| ratatui | TUI framework |
| crossterm | Terminal backend |
| tui-textarea | Text editing widgets |
| notify | Filesystem watching |
| rayon | Parallel iteration |
| toml + serde | Config serialization |
| chrono | Date handling |
| rustyline | Line editing |
| log + env_logger | Logging |

## Testing

- Unit tests: inline `#[cfg(test)]` modules in source files
- Integration tests: `tests/integration_test.rs`
- Test fixtures: `tests/fixtures/*.md` with various markdown task formats
- Use `tempfile` crate (dev-dependency) for tests that need temporary files
