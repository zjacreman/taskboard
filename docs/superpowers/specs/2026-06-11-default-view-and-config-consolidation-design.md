# Default View & Config Consolidation

## Problem

1. `config.defaults.view` is parsed into `Config.defaults.view` but never read. `App::new()` at `src/ui/mod.rs:55` always picks `views.first()` as the startup view.
2. Views live in a separate `views.toml` file — redundant with the `[defaults]` section in `config.toml`.
3. The view manager modal has no way to mark a view as the default.

## Solution

### 1. Consolidate views.toml into config.toml

Move view definitions into `config.toml` under a `[[views]]` array-of-tables section. Remove the standalone `views.toml` file and its loading/saving code.

**New config.toml structure:**

```toml
[workspace]
path = "/path/to/vault"

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

### 2. Wire up `config.defaults.view`

In `App::new()`, look up `config.defaults.view` by name against the views list. Fall back to `views.first()` if no match.

**Before** (`src/ui/mod.rs:55`):
```rust
let current_view = views.first().cloned().unwrap_or_default();
```

**After:**
```rust
let current_view = views.iter()
    .find(|v| v.name == config.defaults.view)
    .or(views.first())
    .cloned()
    .unwrap_or_default();
```

### 3. Add "set default" action to view manager

Press `s` in the view manager to set the selected view as the default. This updates `config.defaults.view` in memory and writes config.toml to disk.

**View manager help line changes:**

Before: `Enter: switch | e: edit | d: del | Esc: close`
After: `Enter: switch | e: edit | d: del | s: set default | Esc: close`

**Visual indicator:** The default view gets a `(default)` suffix after its name in the list. This is distinct from the `* ` prefix used for the currently active view. Example: `  Overdue (default)` vs `* Overdue`.

### 4. Config save support

Add a `Config::save()` method that serializes the full config (workspace, defaults, theme, views) back to the config file path. The `App` struct needs to track which config path was loaded so it can write back to the same file.

## File Changes

| File | Change |
|------|--------|
| `src/config.rs` | Add `views: Vec<ViewConfig>` to `Config`. Add `ViewConfig` struct (name, query, sort_by, group_by). Add `Config::save(&self, path: &Path)`. Add `config_path: PathBuf` field. |
| `src/storage.rs` | Remove `load_views()`, `save_views()`, `ViewsFile`, `ViewSerde`. Keep `save_views` as dead code removal. |
| `src/main.rs` | Remove `views.toml` path resolution and `storage::load_views()` call. Load views from `config.views`. Convert `ViewConfig` to `View`. Pass config path to `App`. |
| `src/ui/mod.rs` | In `App::new()`: match `config.defaults.view` by name. Add `config_path` field to `App`. Replace `views_path` with `config_path`. Add `'s'` handler in `handle_view_manager_key()`. Update `save_views()` to `save_config()` using `Config::save()`. Update view manager drawing to show default marker. Update help line. |
| `src/ui/mod.rs` (draw) | Show default indicator next to default view in list. |
| `README.md` | Update config example to include `[[views]]` section. |

## Detailed Config Structs

```rust
// src/config.rs

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
```

Note: `Config` and sub-structs need `Serialize` derive (currently only `Deserialize`).

## Startup Flow Changes

**Before:**
1. Load config from config.toml
2. Load views from views.toml
3. Create App with config + views
4. App picks `views.first()` as current view

**After:**
1. Load config from config.toml (includes views)
2. Convert `config.views` to `Vec<View>`
3. Create App with config + views
4. App finds view matching `config.defaults.view`, falls back to first

## Migration

Manual. Users edit their config.toml to add `[[views]]` entries and remove the old `~/.config/taskboard/views.toml` file. If `[[views]]` is empty/missing, `App::new()` falls back to `View::default()` (an "All Tasks" view with empty query).

## Test Plan

- `config.rs`: Parse config with `[[views]]` section. Parse config without views (empty vec). Serialize and re-parse roundtrip.
- `ui/mod.rs`: Default view selection from config. Fallback when config default doesn't match any view. `s` key sets default. Config file written after `s`.
- Existing tests updated: `sample_config()` includes views. `test_app()` helper updated for new App signature.

## Out of Scope

- Auto-migration from views.toml
- UI for adding new views (already exists via search + Ctrl+S)
- Editing view query/sort/group from view manager (already exists via `e` key — currently only edits name, could be extended later)
