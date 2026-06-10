# TaskBoard

## The idea
I need a TUI app that can manage the 'task' records that are strewn about in my Obsidian journal. I have started using the obsidian-nvim plugin in Neovim instead of the Obsidian desktop app, and it lacks task management features.

I still like being able to write tasks in whatever note I happen to be working
on. But I require an interface for managing them, querying them, and
filtering them.

## The tech stack
Rust and Ratatui.

Development is happening on Nix and should always use nix-shell. If new packages
are needed, they should be added to shell.nix.

## The requirements
The app should be able to:

- be configurable with an Obsidian workspace
  - config should be saved/read from disk at ~/.config/taskboard/config.toml
    or ./config.toml in that order
- find all of the markdown task checkboxes in that workspace and present
  them to me
- be queryable/filterable on the same terms as Obsidian Tasks
- be able to save queries as 'views' that I can return to
- be able to manage those views
- be able to edit individual tasks
  - simple edits with keyboard shortcuts
  - more complex edits with a modal interface
- be entirely keyboard-driven

## References
Obsidian Tasks source code - github.com/obsidian-tasks-group/obsidian-tasks
Obsidian Tasks user guide - publish.obsidian.md/tasks
