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
