pub mod task_list;
pub mod modal;
pub mod command;

use crate::config::Config;
use crate::task::Task;
use crate::view::View;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

pub struct App {
    pub tasks: Vec<Task>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub views: Vec<View>,
    pub current_view: View,
    pub config: Config,
    pub workspace_path: Option<std::path::PathBuf>,
    pub should_quit: bool,
    pub show_help: bool,
    pub show_modal: bool,
    pub show_view_switcher: bool,
    pub show_view_manager: bool,
    pub search_active: bool,
    pub search_query: String,
    pub saving_view: bool,
    pub view_name_input: String,
    pub view_manager_selected: usize,
    pub editing_view: bool,
    pub editing_view_field: ViewEditField,
    pub editing_view_text: String,
    pub file_watcher: Option<crate::vault::FileWatcher>,
    pub dirty: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewEditField {
    Name,
    Query,
    SortBy,
    GroupBy,
}

impl App {
    pub fn new(config: Config, tasks: Vec<Task>, views: Vec<View>) -> Self {
        let current_view = views.first().cloned().unwrap_or_default();
        let workspace_path = Some(config.workspace.path.clone());
        Self {
            tasks,
            filtered_indices: Vec::new(),
            selected_index: 0,
            views,
            current_view,
            config,
            workspace_path,
            should_quit: false,
            show_help: false,
            show_modal: false,
            show_view_switcher: false,
            show_view_manager: false,
            search_active: false,
            search_query: String::new(),
            saving_view: false,
            view_name_input: String::new(),
            view_manager_selected: 0,
            editing_view: false,
            editing_view_field: ViewEditField::Name,
            editing_view_text: String::new(),
            file_watcher: None,
            dirty: true,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        loop {
            if self.dirty {
                self.update_filtered_tasks();
                self.dirty = false;
            }
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code, key.modifiers);
                    }
                }
            }

            if let Some(watcher) = &self.file_watcher {
                let changed = watcher.poll_changes();
                if !changed.is_empty() {
                    for path in changed {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            self.tasks.retain(|t| t.source_file != path);
                            let new_tasks = crate::task::parser::parse_file(&content, &path);
                            self.tasks.extend(new_tasks);
                        }
                    }
                    self.dirty = true;
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if self.show_help {
            if code == KeyCode::Esc || code == KeyCode::Char('?') {
                self.show_help = false;
            }
            return;
        }
        if self.show_modal {
            modal::handle_key(self, code);
            return;
        }
        if self.search_active {
            command::handle_key(self, code, modifiers);
            return;
        }
        if self.show_view_switcher {
            self.handle_view_switcher_key(code);
            return;
        }
        if self.show_view_manager {
            self.handle_view_manager_key(code);
            return;
        }
        if self.editing_view {
            self.handle_view_manager_key(code);
            return;
        }

        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => self.selected_index = 0,
            KeyCode::Char('G') => self.selected_index = self.filtered_indices.len().saturating_sub(1),
            KeyCode::Char('x') => self.toggle_done(),
            KeyCode::Char('p') => self.cycle_priority(),
            KeyCode::Char('d') => self.set_due_date_today(),
            KeyCode::Char('D') => self.set_due_date_tomorrow(),
            KeyCode::Char('s') => self.set_scheduled_today(),
            KeyCode::Char('S') => self.set_scheduled_tomorrow(),
            KeyCode::Char('b') => self.bump_scheduled(),
            KeyCode::Char('/') => self.start_search(),
            KeyCode::Char('v') => self.show_view_switcher = true,
            KeyCode::Char('V') => self.show_view_manager = true,
            KeyCode::Enter => self.open_modal(),
            KeyCode::Char('r') => self.rescan_vault(),
            _ => {}
        }
    }

    pub fn selected_task_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected_index).copied()
    }

    fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
        }
    }

    fn move_up(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_indices.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    fn toggle_done(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].status.cycle();
            self.dirty = true;
        }
    }

    fn cycle_priority(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].priority.cycle();
            self.dirty = true;
        }
    }

    fn set_due_date_today(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].due_date = Some(chrono::Local::now().date_naive());
            self.dirty = true;
        }
    }

    fn set_due_date_tomorrow(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].due_date = Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
            self.dirty = true;
        }
    }

    fn set_scheduled_today(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].scheduled_date = Some(chrono::Local::now().date_naive());
            self.dirty = true;
        }
    }

    fn set_scheduled_tomorrow(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].scheduled_date = Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
            self.dirty = true;
        }
    }

    fn bump_scheduled(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            let date = self.tasks[idx].scheduled_date.unwrap_or_else(|| chrono::Local::now().date_naive());
            self.tasks[idx].scheduled_date = Some(date + chrono::Duration::days(1));
            self.dirty = true;
        }
    }

    fn start_search(&mut self) {
        self.search_active = true;
        self.search_query = self.current_view.query.clone();
    }

    fn open_modal(&mut self) {
        if self.selected_task_index().is_some() {
            self.show_modal = true;
        }
    }

    fn rescan_vault(&mut self) {
        let workspace_path = self.config.workspace.path.clone();
        let md_files = crate::vault::find_markdown_files(&workspace_path);
        self.tasks.clear();
        for file in &md_files {
            if let Ok(content) = std::fs::read_to_string(file) {
                let tasks = crate::task::parser::parse_file(&content, file);
                self.tasks.extend(tasks);
            }
        }
        self.dirty = true;
    }

    fn handle_view_switcher_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.show_view_switcher = false,
            KeyCode::Char('j') | KeyCode::Down => {
                // TODO: cycle through views
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // TODO: cycle through views
            }
            KeyCode::Enter => {
                // TODO: select view
                self.show_view_switcher = false;
            }
            _ => {}
        }
    }

    fn handle_view_manager_key(&mut self, code: KeyCode) {
        if self.editing_view {
            match code {
                KeyCode::Esc => {
                    self.editing_view = false;
                    self.editing_view_text.clear();
                }
                KeyCode::Enter => {
                    // Save the edited field
                    if let Some(view) = self.views.get_mut(self.view_manager_selected) {
                        match self.editing_view_field {
                            ViewEditField::Name => view.name = self.editing_view_text.clone(),
                            ViewEditField::Query => view.query = self.editing_view_text.clone(),
                            ViewEditField::SortBy => view.sort_by = self.editing_view_text.clone(),
                            ViewEditField::GroupBy => view.group_by = self.editing_view_text.clone(),
                        }
                    }
                    self.editing_view = false;
                    self.editing_view_text.clear();
                    self.save_views();
                }
                KeyCode::Backspace => {
                    self.editing_view_text.pop();
                }
                KeyCode::Char(c) => {
                    self.editing_view_text.push(c);
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc => {
                self.show_view_manager = false;
            }
            KeyCode::Char('j') | KeyCode::Down
                if !self.views.is_empty() =>
            {
                self.view_manager_selected = (self.view_manager_selected + 1) % self.views.len();
            }
            KeyCode::Char('k') | KeyCode::Up
                if !self.views.is_empty() =>
            {
                self.view_manager_selected = if self.view_manager_selected == 0 {
                    self.views.len() - 1
                } else {
                    self.view_manager_selected - 1
                };
            }
            KeyCode::Char('e') => {
                // Edit selected view
                if let Some(view) = self.views.get(self.view_manager_selected) {
                    self.editing_view = true;
                    self.editing_view_field = ViewEditField::Name;
                    self.editing_view_text = view.name.clone();
                }
            }
            KeyCode::Tab if self.editing_view => {
                // Cycle through edit fields when editing
                self.editing_view_field = match self.editing_view_field {
                    ViewEditField::Name => ViewEditField::Query,
                    ViewEditField::Query => ViewEditField::SortBy,
                    ViewEditField::SortBy => ViewEditField::GroupBy,
                    ViewEditField::GroupBy => ViewEditField::Name,
                };
                // Load the current value of the new field
                if let Some(view) = self.views.get(self.view_manager_selected) {
                    self.editing_view_text = match self.editing_view_field {
                        ViewEditField::Name => view.name.clone(),
                        ViewEditField::Query => view.query.clone(),
                        ViewEditField::SortBy => view.sort_by.clone(),
                        ViewEditField::GroupBy => view.group_by.clone(),
                    };
                }
            }
            KeyCode::Char('d')
                if self.views.len() > 1 && self.view_manager_selected < self.views.len() =>
            {
                // Delete selected view (but not the last one)
                self.views.remove(self.view_manager_selected);
                if self.view_manager_selected >= self.views.len() {
                    self.view_manager_selected = self.views.len() - 1;
                }
                self.save_views();
            }
            KeyCode::Enter => {
                // Switch to selected view
                if let Some(view) = self.views.get(self.view_manager_selected) {
                    self.current_view = view.clone();
                    self.show_view_manager = false;
                    self.dirty = true;
                }
            }
            _ => {}
        }
    }

    fn save_views(&self) {
        let views_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("taskboard")
            .join("views.toml");
        if let Err(e) = crate::storage::save_views(&self.views, &views_path) {
            log::warn!("Failed to save views: {}", e);
        }
    }

    pub fn update_filtered_tasks(&mut self) {
        let query = if self.search_active && !self.search_query.is_empty() {
            &self.search_query
        } else {
            &self.current_view.query
        };

        self.filtered_indices = crate::task::query::execute_query(query, &self.tasks)
            .unwrap_or_default()
            .iter()
            .map(|t| {
                self.tasks.iter().position(|task| {
                    task.source_file == t.source_file && task.line_number == t.line_number
                }).unwrap_or(0)
            })
            .collect();

        // Clamp selected_index
        if !self.filtered_indices.is_empty() && self.selected_index >= self.filtered_indices.len() {
            self.selected_index = self.filtered_indices.len() - 1;
        }
    }

    fn draw(&self, frame: &mut ratatui::Frame) {
        task_list::draw(frame, self);

        if self.show_help {
            draw_help_overlay(frame);
        }
        if self.show_modal {
            modal::draw(frame, self);
        }
        if self.search_active {
            command::draw(frame, self);
        }
        if self.show_view_switcher {
            draw_view_switcher(frame, self);
        }
        if self.show_view_manager || self.editing_view {
            draw_view_manager(frame, self);
        }
    }
}

fn draw_view_switcher(frame: &mut ratatui::Frame, app: &App) {
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};
    use ratatui::text::Line;
    use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

    let area = frame.area();
    let popup_width = 40.min(area.width - 4);
    let popup_height = (app.views.len() as u16 + 3).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app
        .views
        .iter()
        .map(|v| {
            let style = if v.name == app.current_view.name {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(v.name.clone())).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Switch View")
            .style(Style::default().bg(Color::DarkGray)),
    );

    frame.render_widget(list, popup_area);
}

fn draw_view_manager(frame: &mut ratatui::Frame, app: &App) {
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

    let area = frame.area();
    let popup_width = 60.min(area.width - 4);
    let popup_height = (app.views.len() as u16 + 8).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    if app.editing_view {
        // Show edit interface
        let field_name = match app.editing_view_field {
            ViewEditField::Name => "Name",
            ViewEditField::Query => "Query",
            ViewEditField::SortBy => "Sort By",
            ViewEditField::GroupBy => "Group By",
        };

        let content = vec![
            Line::from(format!("Editing view: {}", app.views.get(app.view_manager_selected).map(|v| v.name.as_str()).unwrap_or(""))),
            Line::from(""),
            Line::from(vec![
                Span::styled(format!("{}: ", field_name), Style::default().fg(Color::DarkGray)),
                Span::raw(&app.editing_view_text),
                Span::styled("█", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Tab: next field | Enter: save | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Edit View")
                    .style(Style::default().bg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(paragraph, popup_area);
    } else {
        // Show view list
        let items: Vec<ListItem> = app
            .views
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let style = if i == app.view_manager_selected {
                    Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
                } else if v.name == app.current_view.name {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                let prefix = if v.name == app.current_view.name { "* " } else { "  " };
                ListItem::new(Line::from(format!("{}{}", prefix, v.name))).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Manage Views")
                .style(Style::default().bg(Color::DarkGray)),
        );

        frame.render_stateful_widget(list, popup_area, &mut ratatui::widgets::ListState::default().with_offset(app.view_manager_selected));

        // Draw help text below the list
        let help_area = Rect::new(popup_area.x + 1, popup_area.y + popup_area.height - 2, popup_area.width - 2, 1);
        let help = Paragraph::new(Line::from(Span::styled(
            "Enter: switch | e: edit | d: delete | Esc: close",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(help, help_area);
    }
}

fn draw_help_overlay(frame: &mut ratatui::Frame) {
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};
    use ratatui::text::Line;
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let area = frame.area();
    let popup_width = 50.min(area.width - 4);
    let popup_height = 20.min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from("Navigation:"),
        Line::from("  j/k, ↑/↓  Move up/down"),
        Line::from("  g/G       Jump to top/bottom"),
        Line::from(""),
        Line::from("Quick Actions:"),
        Line::from("  x         Toggle done"),
        Line::from("  p         Cycle priority"),
        Line::from("  d/D       Due date: today/tomorrow"),
        Line::from("  s/S       Scheduled: today/tomorrow"),
        Line::from("  b         Bump scheduled +1 day"),
        Line::from(""),
        Line::from("Views:"),
        Line::from("  /         Search/query"),
        Line::from("  v         Switch view"),
        Line::from("  V         Manage views (e:edit, d:delete)"),
        Line::from(""),
        Line::from("  Enter     Edit task (modal)"),
        Line::from("  r         Rescan vault"),
        Line::from("  ?         Toggle help"),
        Line::from("  q         Quit"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .style(Style::default().bg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, WorkspaceConfig, DefaultsConfig, ThemeConfig};
    use crate::task::{TaskStatus, Priority};
    use crate::view::View;
    use crate::test_helpers::sample_tasks;
    use std::path::PathBuf;

    use super::App;

    fn sample_config() -> Config {
        Config {
            workspace: WorkspaceConfig { path: PathBuf::from(".") },
            defaults: DefaultsConfig::default(),
            theme: ThemeConfig::default(),
        }
    }

    fn sample_views() -> Vec<View> {
        vec![View::default()]
    }

    #[test]
    fn test_app_new() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let app = App::new(config, tasks, views);

        assert_eq!(app.selected_index, 0);
        assert!(!app.should_quit);
        assert!(!app.show_help);
        assert_eq!(app.current_view.name, "All Tasks");
    }

    #[test]
    fn test_move_down() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.selected_index, 0);
        app.move_down();
        assert_eq!(app.selected_index, 1);
        app.move_down();
        assert_eq!(app.selected_index, 2);
        app.move_down();
        assert_eq!(app.selected_index, 0); // wraps
    }

    #[test]
    fn test_move_up() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.selected_index, 0);
        app.move_up();
        assert_eq!(app.selected_index, 2); // wraps to end
        app.move_up();
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn test_move_down_empty() {
        let config = sample_config();
        let views = sample_views();
        let mut app = App::new(config, vec![], views);
        app.update_filtered_tasks();

        app.move_down();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_move_up_empty() {
        let config = sample_config();
        let views = sample_views();
        let mut app = App::new(config, vec![], views);
        app.update_filtered_tasks();

        app.move_up();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_toggle_done() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.tasks[0].status, TaskStatus::Todo);
        app.toggle_done();
        assert_eq!(app.tasks[0].status, TaskStatus::Done);
        app.toggle_done();
        assert_eq!(app.tasks[0].status, TaskStatus::Todo);
    }

    #[test]
    fn test_cycle_priority() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.tasks[0].priority, Priority::None);
        app.cycle_priority();
        assert_eq!(app.tasks[0].priority, Priority::High);
    }

    #[test]
    fn test_set_due_date_today() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        app.selected_index = 2;
        app.set_due_date_today();
        assert_eq!(app.tasks[2].due_date, Some(chrono::Local::now().date_naive()));
    }

    #[test]
    fn test_set_due_date_tomorrow() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        app.set_due_date_tomorrow();
        let tomorrow = chrono::Local::now().date_naive() + chrono::Duration::days(1);
        assert_eq!(app.tasks[0].due_date, Some(tomorrow));
    }

    #[test]
    fn test_set_scheduled_today() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        app.set_scheduled_today();
        assert_eq!(app.tasks[0].scheduled_date, Some(chrono::Local::now().date_naive()));
    }

    #[test]
    fn test_set_scheduled_tomorrow() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        app.set_scheduled_tomorrow();
        let tomorrow = chrono::Local::now().date_naive() + chrono::Duration::days(1);
        assert_eq!(app.tasks[0].scheduled_date, Some(tomorrow));
    }

    #[test]
    fn test_bump_scheduled() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        app.selected_index = 2;
        let original = app.tasks[2].scheduled_date.unwrap();
        app.bump_scheduled();
        assert_eq!(app.tasks[2].scheduled_date, Some(original + chrono::Duration::days(1)));
    }

    #[test]
    fn test_bump_scheduled_no_existing() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        app.selected_index = 0;
        app.bump_scheduled();
        let today = chrono::Local::now().date_naive();
        assert_eq!(app.tasks[0].scheduled_date, Some(today + chrono::Duration::days(1)));
    }

    #[test]
    fn test_update_filtered_tasks() {
        let config = sample_config();
        let tasks = sample_tasks();
        let mut views = sample_views();
        views[0].query = "not done".to_string();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.filtered_indices.len(), 2);
    }

    #[test]
    fn test_update_filtered_tasks_empty_query() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.filtered_indices.len(), 3);
    }

    #[test]
    fn test_default_view_used_when_empty() {
        let config = sample_config();
        let tasks = sample_tasks();
        let app = App::new(config, tasks, vec![]);

        assert_eq!(app.current_view.name, "All Tasks");
    }

    #[test]
    fn test_mutations_persist() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        app.toggle_done();
        assert_eq!(app.tasks[0].status, TaskStatus::Done);

        app.update_filtered_tasks();
        assert_eq!(app.tasks[0].status, TaskStatus::Done);

        app.cycle_priority();
        assert_eq!(app.tasks[0].priority, Priority::High);

        app.update_filtered_tasks();
        assert_eq!(app.tasks[0].priority, Priority::High);
    }
}
