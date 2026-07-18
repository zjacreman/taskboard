pub mod filter;
pub mod modal;
pub mod task_list;
pub mod view_manager;

use crate::config::Config;
use crate::task::Task;
use crate::view::View;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use tui_textarea::{CursorMove, TextArea};

use crate::ui::modal::EditField as TaskEditField;
use ratatui::widgets::ListState;

pub struct App {
    pub tasks: Vec<Task>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub views: Vec<View>,
    pub current_view: View,
    pub config: Config,
    pub workspace_path: Option<std::path::PathBuf>,
    pub config_path: std::path::PathBuf,
    pub should_quit: bool,
    pub show_help: bool,
    pub show_modal: bool,
    pub show_view_manager: bool,
    pub view_manager_state: ListState,
    pub view_edit: Option<TextArea<'static>>,
    pub task_edit: Option<TextArea<'static>>,
    pub editing_view_field: ViewEditField,
    pub task_edit_field: TaskEditField,
    pub file_watcher: Option<crate::vault::FileWatcher>,
    pub dirty: bool,
    pub status_message: Option<String>,
    pub filter_text: String,
    pub filter_textarea: Option<TextArea<'static>>,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ViewEditField {
    Name,
    Query,
    SortBy,
    GroupBy,
}

impl App {
    pub fn new(
        config: Config,
        tasks: Vec<Task>,
        views: Vec<View>,
        config_path: std::path::PathBuf,
    ) -> Self {
        let current_view = views
            .iter()
            .find(|v| v.name == config.defaults.view)
            .or(views.first())
            .cloned()
            .unwrap_or_default();
        let workspace_path = Some(config.workspace.path.clone());
        let mut view_manager_state = ListState::default();
        if !views.is_empty() {
            view_manager_state.select(Some(0));
        }
        Self {
            tasks,
            filtered_indices: Vec::new(),
            selected_index: 0,
            views,
            current_view,
            config,
            workspace_path,
            config_path,
            should_quit: false,
            show_help: false,
            show_modal: false,
            show_view_manager: false,
            view_manager_state,
            view_edit: None,
            task_edit: None,
            editing_view_field: ViewEditField::Name,
            task_edit_field: TaskEditField::Description,
            file_watcher: None,
            dirty: true,
            status_message: None,
            filter_text: String::new(),
            filter_textarea: None,
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
                        self.handle_key(key);
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

    fn handle_key(&mut self, key: KeyEvent) {
        let code = key.code;
        if self.show_help {
            if code == KeyCode::Esc || code == KeyCode::Char('?') {
                self.show_help = false;
            }
            return;
        }
        if self.show_modal {
            modal::handle_key(self, key);
            return;
        }
        if self.filter_textarea.is_some() {
            filter::handle_key(self, key);
            return;
        }
        if self.show_view_manager {
            self.handle_view_manager_key(key);
            return;
        }

        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => self.selected_index = 0,
            KeyCode::Char('G') => {
                self.selected_index = self.filtered_indices.len().saturating_sub(1)
            }
            KeyCode::Char('x') => self.toggle_done(),
            KeyCode::Char('p') => self.cycle_priority(),
            KeyCode::Char('d') => self.set_due_date_today(),
            KeyCode::Char('D') => self.set_due_date_tomorrow(),
            KeyCode::Char('s') => self.set_scheduled_today(),
            KeyCode::Char('S') => self.set_scheduled_tomorrow(),
            KeyCode::Char('b') => self.bump_scheduled(),
            KeyCode::Char('/') => self.start_filter(),
            KeyCode::Esc if !self.filter_text.is_empty() => {
                self.filter_text.clear();
                self.dirty = true;
            }
            KeyCode::Char('v') => self.show_view_manager = true,
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

    pub fn persist_task(&mut self, idx: usize) {
        if let Some(task) = self.tasks.get(idx) {
            match task.write_to_file() {
                Ok(()) => {
                    self.status_message = None;
                }
                Err(e) => {
                    let msg = format!("Failed to save: {}", e);
                    log::warn!("{}", msg);
                    self.status_message = Some(msg);
                }
            }
        }
    }

    fn toggle_done(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].status.cycle();
            match self.tasks[idx].status {
                crate::task::TaskStatus::Done => {
                    self.tasks[idx].done_date = Some(chrono::Local::now().date_naive());
                }
                crate::task::TaskStatus::Todo => {
                    self.tasks[idx].done_date = None;
                }
            }
            self.persist_task(idx);
            self.dirty = true;
        }
    }

    fn cycle_priority(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].priority.cycle();
            self.persist_task(idx);
            self.dirty = true;
        }
    }

    fn set_due_date_today(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].due_date = Some(chrono::Local::now().date_naive());
            self.persist_task(idx);
            self.dirty = true;
        }
    }

    fn set_due_date_tomorrow(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].due_date =
                Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
            self.persist_task(idx);
            self.dirty = true;
        }
    }

    fn set_scheduled_today(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].scheduled_date = Some(chrono::Local::now().date_naive());
            self.persist_task(idx);
            self.dirty = true;
        }
    }

    fn set_scheduled_tomorrow(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            self.tasks[idx].scheduled_date =
                Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
            self.persist_task(idx);
            self.dirty = true;
        }
    }

    fn bump_scheduled(&mut self) {
        if let Some(idx) = self.selected_task_index() {
            let date = self.tasks[idx]
                .scheduled_date
                .unwrap_or_else(|| chrono::Local::now().date_naive());
            self.tasks[idx].scheduled_date = Some(date + chrono::Duration::days(1));
            self.persist_task(idx);
            self.dirty = true;
        }
    }

    fn start_filter(&mut self) {
        let mut textarea = TextArea::new(vec![self.filter_text.clone()]);
        textarea.set_cursor_line_style(ratatui::style::Style::default());
        textarea.move_cursor(CursorMove::End);
        self.filter_textarea = Some(textarea);
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

    pub fn handle_view_manager_key(&mut self, key: KeyEvent) {
        view_manager::handle_key(self, key);
    }

    fn save_config(&mut self) {
        self.config.views = self
            .views
            .iter()
            .map(|v| crate::config::ViewConfig {
                name: v.name.clone(),
                query: v.query.clone(),
                sort_by: v.sort_by.clone(),
                group_by: v.group_by.clone(),
            })
            .collect();
        if let Err(e) = self.config.save(&self.config_path) {
            log::warn!("Failed to save config: {}", e);
        }
    }

    pub fn update_filtered_tasks(&mut self) {
        let mut result = crate::task::query::execute_query(&self.current_view.query, &self.tasks)
            .unwrap_or_default();

        // Default sort: by source file path, then by line number
        result.sort_by(|a, b| {
            a.source_file
                .cmp(&b.source_file)
                .then(a.line_number.cmp(&b.line_number))
        });

        self.filtered_indices = result
            .iter()
            .filter_map(|t| {
                self.tasks.iter().position(|task| {
                    task.source_file == t.source_file && task.line_number == t.line_number
                })
            })
            .collect();

        self.filtered_indices
            .retain(|&idx| filter::matches_filter(&self.tasks[idx], &self.filter_text));

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
        if self.filter_textarea.is_some() {
            filter::draw(frame, self);
        }
        if self.show_view_manager || self.view_edit.is_some() {
            view_manager::draw(frame, self);
        }
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
        Line::from("  /         Filter tasks (text)"),
        Line::from("  Esc       Clear active filter"),
        Line::from("  v         Manage views (a:add e:edit d:del s:default)"),
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
    use crate::config::{Config, DefaultsConfig, ThemeConfig, WorkspaceConfig};
    use crate::task::{Priority, Task, TaskStatus};
    use crate::test_helpers::sample_tasks;
    use crate::ui::modal::EditField;
    use crate::view::View;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;

    use super::App;

    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn sample_config() -> Config {
        Config {
            workspace: WorkspaceConfig {
                path: PathBuf::from("."),
            },
            defaults: DefaultsConfig::default(),
            theme: ThemeConfig::default(),
            views: vec![],
        }
    }

    fn sample_views() -> Vec<View> {
        vec![View::default()]
    }

    fn test_app(tasks: Vec<Task>, views: Vec<View>) -> App {
        let config = sample_config();
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let app = App::new(config, tasks, views, config_path);
        // Leak the dir so it doesn't get deleted during the test
        std::mem::forget(dir);
        app
    }

    #[test]
    fn test_app_new() {
        let tasks = sample_tasks();
        let views = sample_views();
        let app = test_app(tasks, views);

        assert_eq!(app.selected_index, 0);
        assert!(!app.should_quit);
        assert!(!app.show_help);
        assert_eq!(app.current_view.name, "All Tasks");
    }

    #[test]
    fn test_move_down() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
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
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.selected_index, 0);
        app.move_up();
        assert_eq!(app.selected_index, 2); // wraps to end
        app.move_up();
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn test_move_down_empty() {
        let views = sample_views();
        let mut app = test_app(vec![], views);
        app.update_filtered_tasks();

        app.move_down();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_move_up_empty() {
        let views = sample_views();
        let mut app = test_app(vec![], views);
        app.update_filtered_tasks();

        app.move_up();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_toggle_done() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        // tasks[0] is "Fix bug" (Todo) — first in sorted order
        assert_eq!(app.tasks[0].status, TaskStatus::Todo);
        app.toggle_done();
        assert_eq!(app.tasks[0].status, TaskStatus::Done);
        app.toggle_done();
        assert_eq!(app.tasks[0].status, TaskStatus::Todo);
    }

    #[test]
    fn test_cycle_priority() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        // tasks[0] is "Fix bug" (priority Medium)
        assert_eq!(app.tasks[0].priority, Priority::Medium);
        app.cycle_priority();
        assert_eq!(app.tasks[0].priority, Priority::Low);
    }

    #[test]
    fn test_set_due_date_today() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        app.selected_index = 2;
        let idx = app.selected_task_index().unwrap();
        app.set_due_date_today();
        assert_eq!(
            app.tasks[idx].due_date,
            Some(chrono::Local::now().date_naive())
        );
    }

    #[test]
    fn test_set_due_date_tomorrow() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        let idx = app.selected_task_index().unwrap();
        app.set_due_date_tomorrow();
        let tomorrow = chrono::Local::now().date_naive() + chrono::Duration::days(1);
        assert_eq!(app.tasks[idx].due_date, Some(tomorrow));
    }

    #[test]
    fn test_set_scheduled_today() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        let idx = app.selected_task_index().unwrap();
        app.set_scheduled_today();
        assert_eq!(
            app.tasks[idx].scheduled_date,
            Some(chrono::Local::now().date_naive())
        );
    }

    #[test]
    fn test_set_scheduled_tomorrow() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        let idx = app.selected_task_index().unwrap();
        app.set_scheduled_tomorrow();
        let tomorrow = chrono::Local::now().date_naive() + chrono::Duration::days(1);
        assert_eq!(app.tasks[idx].scheduled_date, Some(tomorrow));
    }

    #[test]
    fn test_bump_scheduled() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        app.selected_index = 2;
        let idx = app.selected_task_index().unwrap();
        let original = app.tasks[idx].scheduled_date.unwrap();
        app.bump_scheduled();
        assert_eq!(
            app.tasks[idx].scheduled_date,
            Some(original + chrono::Duration::days(1))
        );
    }

    #[test]
    fn test_bump_scheduled_no_existing() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        // tasks[1] = "Buy groceries" has no scheduled_date
        app.selected_index = 1;
        let idx = app.selected_task_index().unwrap();
        assert!(app.tasks[idx].scheduled_date.is_none());
        app.bump_scheduled();
        let today = chrono::Local::now().date_naive();
        assert_eq!(
            app.tasks[idx].scheduled_date,
            Some(today + chrono::Duration::days(1))
        );
    }

    #[test]
    fn test_update_filtered_tasks() {
        let tasks = sample_tasks();
        let mut views = sample_views();
        views[0].query = "not done".to_string();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.filtered_indices.len(), 2);
    }

    #[test]
    fn test_update_filtered_tasks_empty_query() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.filtered_indices.len(), 3);
    }

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

    #[test]
    fn test_default_view_used_when_empty() {
        let tasks = sample_tasks();
        let app = test_app(tasks, vec![]);

        assert_eq!(app.current_view.name, "All Tasks");
    }

    #[test]
    fn test_mutations_persist() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        let idx = app.selected_task_index().unwrap();
        assert_eq!(app.tasks[idx].status, TaskStatus::Todo);
        app.toggle_done();
        assert_eq!(app.tasks[idx].status, TaskStatus::Done);

        app.update_filtered_tasks();
        let idx = app.selected_task_index().unwrap();
        assert_eq!(app.tasks[idx].status, TaskStatus::Done);

        assert_eq!(app.tasks[idx].priority, Priority::Medium);
        app.cycle_priority();
        assert_eq!(app.tasks[idx].priority, Priority::Low);

        app.update_filtered_tasks();
        let idx = app.selected_task_index().unwrap();
        assert_eq!(app.tasks[idx].priority, Priority::Low);
    }

    #[test]
    fn test_view_manager_navigation() {
        let tasks = sample_tasks();
        let views = vec![
            View::new("View1", "not done", "", ""),
            View::new("View2", "done", "", ""),
            View::new("View3", "", "", ""),
        ];
        let mut app = test_app(tasks, views);
        app.show_view_manager = true;

        assert_eq!(app.view_manager_state.selected(), Some(0));

        app.handle_view_manager_key(key_event(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.view_manager_state.selected(), Some(1));

        app.handle_view_manager_key(key_event(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.view_manager_state.selected(), Some(2));

        app.handle_view_manager_key(key_event(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.view_manager_state.selected(), Some(0)); // wraps

        app.handle_view_manager_key(key_event(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.view_manager_state.selected(), Some(2)); // wraps back
    }

    #[test]
    fn test_view_manager_select() {
        let tasks = sample_tasks();
        let views = vec![
            View::new("View1", "not done", "", ""),
            View::new("View2", "done", "", ""),
        ];
        let mut app = test_app(tasks, views);
        app.show_view_manager = true;

        app.handle_view_manager_key(key_event(KeyCode::Down, KeyModifiers::NONE));
        app.handle_view_manager_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.current_view.name, "View2");
        assert!(!app.show_view_manager);
    }

    #[test]
    fn test_view_manager_delete() {
        let tasks = sample_tasks();
        let views = vec![
            View::new("View1", "not done", "", ""),
            View::new("View2", "done", "", ""),
            View::new("View3", "", "", ""),
        ];
        let mut app = test_app(tasks, views);
        app.show_view_manager = true;

        app.handle_view_manager_key(key_event(KeyCode::Down, KeyModifiers::NONE)); // select View2
        app.handle_view_manager_key(key_event(KeyCode::Char('d'), KeyModifiers::NONE)); // delete View2
        assert_eq!(app.views.len(), 2);
        assert_eq!(app.views[0].name, "View1");
        assert_eq!(app.views[1].name, "View3");
    }

    #[test]
    fn test_view_manager_esc() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.show_view_manager = true;

        app.handle_view_manager_key(key_event(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.show_view_manager);
    }

    #[test]
    fn test_task_edit_modal_fields() {
        use crate::ui::modal::EditField;

        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        assert_eq!(app.task_edit_field, EditField::Description);

        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::Status);

        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::Priority);

        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::DueDate);

        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::ScheduledDate);

        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::Recurrence);

        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::Description); // wraps
    }

    #[test]
    fn test_task_edit_backward_navigation() {
        use crate::ui::modal::EditField;

        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        assert_eq!(app.task_edit_field, EditField::Description);

        app.handle_key(key_event(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(app.task_edit_field, EditField::Recurrence); // wraps backward
    }

    #[test]
    fn test_task_edit_field_edit() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();
        let original_desc = app.tasks[idx].description.clone();

        // Start editing description
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.task_edit.is_some());
        assert_eq!(app.task_edit.as_ref().unwrap().lines()[0], original_desc);

        // Clear the field
        for _ in 0..original_desc.len() {
            app.handle_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        }
        assert_eq!(app.task_edit.as_ref().unwrap().lines()[0], "");

        // Type new description
        for c in "Cook".chars() {
            app.handle_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
        }

        // Save
        app.handle_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.task_edit.is_none());
        assert_eq!(app.tasks[idx].description, "Cook");
    }

    #[test]
    fn test_task_edit_cancel() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();
        let original_desc = app.tasks[idx].description.clone();

        // Start editing description
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.task_edit.is_some());

        // Cancel
        app.handle_key(key_event(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.task_edit.is_none());
        assert_eq!(app.tasks[idx].description, original_desc); // unchanged
    }

    #[test]
    fn test_task_edit_status_field() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();
        let original_status = app.tasks[idx].status;
        let status_text = match original_status {
            TaskStatus::Todo => "todo",
            TaskStatus::Done => "done",
        };

        // Navigate to status field
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::Status);

        // Start editing
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(app.task_edit.as_ref().unwrap().lines()[0], status_text);

        // Clear and type opposite status
        let opposite = match original_status {
            TaskStatus::Todo => "done",
            TaskStatus::Done => "todo",
        };
        for _ in 0..status_text.len() {
            app.handle_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        }
        for c in opposite.chars() {
            app.handle_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
        }

        // Save
        app.handle_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        let expected = match original_status {
            TaskStatus::Todo => TaskStatus::Done,
            TaskStatus::Done => TaskStatus::Todo,
        };
        assert_eq!(app.tasks[idx].status, expected);
    }

    #[test]
    fn test_task_edit_priority_field() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();
        let original_priority = app.tasks[idx].priority;

        // Navigate to priority field
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::Priority);

        // Press 'e' to cycle priority (picker, not text editor)
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        // Priority should have cycled from original
        assert_ne!(app.tasks[idx].priority, original_priority);
        // Should not have opened a text editor
        assert!(app.task_edit.is_none());
    }

    #[test]
    fn test_task_edit_date_field() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();
        let original_due = app.tasks[idx].due_date;

        // Navigate to due date field
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::DueDate);

        // Start editing
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        let edit_text = app.task_edit.as_ref().unwrap().lines()[0].clone();
        let expected_edit = original_due
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        assert_eq!(edit_text, expected_edit);

        // Clear and type "tomorrow"
        for _ in 0..edit_text.len() {
            app.handle_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        }
        for c in "tomorrow".chars() {
            app.handle_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
        }

        // Save
        app.handle_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        let tomorrow = chrono::Local::now().date_naive() + chrono::Duration::days(1);
        assert_eq!(app.tasks[idx].due_date, Some(tomorrow));
    }

    #[test]
    fn test_task_edit_clear_date() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();

        // Navigate to due date field
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.task_edit_field, EditField::DueDate);

        // Start editing
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));

        // Type "none"
        for c in "none".chars() {
            app.handle_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
        }

        // Save
        app.handle_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.tasks[idx].due_date, None);
    }

    #[test]
    fn test_task_edit_recurrence_field() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();
        let original_recurrence = app.tasks[idx].recurrence.clone();

        // Navigate to recurrence field
        for _ in 0..5 {
            app.handle_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        }
        assert_eq!(app.task_edit_field, EditField::Recurrence);

        // Start editing
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(
            app.task_edit.as_ref().unwrap().lines()[0],
            original_recurrence.as_deref().unwrap_or("")
        );

        // Clear and type "every month"
        let clear_len = original_recurrence.as_ref().map(|s| s.len()).unwrap_or(0);
        for _ in 0..clear_len {
            app.handle_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        }
        for c in "every month".chars() {
            app.handle_key(key_event(KeyCode::Char(c), KeyModifiers::NONE));
        }

        // Save
        app.handle_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.tasks[idx].recurrence, Some("every month".to_string()));
    }

    #[test]
    fn test_task_edit_cursor_insert() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();
        let original_desc = app.tasks[idx].description.clone();

        // Start editing description
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.task_edit.is_some());

        // Move cursor left twice (from end)
        app.handle_key(key_event(KeyCode::Left, KeyModifiers::NONE));
        app.handle_key(key_event(KeyCode::Left, KeyModifiers::NONE));

        // Insert 'x' at cursor position
        app.handle_key(key_event(KeyCode::Char('x'), KeyModifiers::NONE));

        // Verify text has 'x' inserted two chars from end
        let text = &app.task_edit.as_ref().unwrap().lines()[0];
        let expected = format!(
            "{}x{}",
            &original_desc[..original_desc.len() - 2],
            &original_desc[original_desc.len() - 2..]
        );
        assert_eq!(text, &expected);

        // Save
        app.handle_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.tasks[idx].description, expected);
    }

    #[test]
    fn test_task_edit_home_end() {
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();
        app.show_modal = true;

        let idx = app.selected_task_index().unwrap();
        let original_desc = app.tasks[idx].description.clone();

        // Start editing description
        app.handle_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));

        // Move to start
        app.handle_key(key_event(KeyCode::Home, KeyModifiers::NONE));

        // Insert at start
        app.handle_key(key_event(KeyCode::Char('!'), KeyModifiers::NONE));

        // Verify
        let text = &app.task_edit.as_ref().unwrap().lines()[0];
        assert_eq!(text, &format!("!{}", original_desc));
    }

    #[test]
    fn test_toggle_done_persists_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("persist_test.md");
        std::fs::write(
            &file_path,
            "# Tasks\n- [ ] Buy groceries\n- [x] Review PR\n",
        )
        .unwrap();

        let tasks = vec![
            Task {
                description: "Buy groceries".to_string(),
                status: TaskStatus::Todo,
                priority: Priority::None,
                due_date: None,
                scheduled_date: None,
                recurrence: None,
                done_date: None,
                start_date: None,
                tags: vec![],
                source_file: file_path.clone(),
                line_number: 2,
            },
            Task {
                description: "Review PR".to_string(),
                status: TaskStatus::Done,
                priority: Priority::None,
                due_date: None,
                scheduled_date: None,
                recurrence: None,
                done_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
                start_date: None,
                tags: vec![],
                source_file: file_path.clone(),
                line_number: 3,
            },
        ];

        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        // Toggle first task to done
        app.toggle_done();
        assert_eq!(app.tasks[0].status, TaskStatus::Done);

        // Verify file was updated
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(
            content.contains("- [x] Buy groceries"),
            "File should contain done task, got: {}",
            content
        );
        assert!(
            content.contains("✅"),
            "File should contain done date emoji, got: {}",
            content
        );
    }

    #[test]
    fn test_toggle_done_then_rescan() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rescan_test.md");
        std::fs::write(&file_path, "# Tasks\n- [ ] Buy groceries\n").unwrap();

        let tasks = vec![Task {
            description: "Buy groceries".to_string(),
            status: TaskStatus::Todo,
            priority: Priority::None,
            due_date: None,
            scheduled_date: None,
            recurrence: None,
            done_date: None,
            start_date: None,
            tags: vec![],
            source_file: file_path.clone(),
            line_number: 2,
        }];

        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        // Toggle to done
        app.toggle_done();
        assert_eq!(app.tasks[0].status, TaskStatus::Done);

        // Simulate rescan (re-read from disk)
        let content = std::fs::read_to_string(&file_path).unwrap();
        let reparsed = crate::task::parser::parse_file(&content, &file_path);
        assert_eq!(reparsed.len(), 1);
        assert_eq!(
            reparsed[0].status,
            TaskStatus::Done,
            "Rescanned task should be done"
        );
    }

    #[test]
    fn test_sorted_by_path_then_line() {
        let tasks = vec![
            Task {
                description: "Zebra task".to_string(),
                status: TaskStatus::Todo,
                priority: Priority::None,
                due_date: None,
                scheduled_date: None,
                recurrence: None,
                done_date: None,
                start_date: None,
                tags: vec![],
                source_file: PathBuf::from("b.md"),
                line_number: 1,
            },
            Task {
                description: "Alpha task".to_string(),
                status: TaskStatus::Todo,
                priority: Priority::None,
                due_date: None,
                scheduled_date: None,
                recurrence: None,
                done_date: None,
                start_date: None,
                tags: vec![],
                source_file: PathBuf::from("a.md"),
                line_number: 1,
            },
            Task {
                description: "Another task".to_string(),
                status: TaskStatus::Todo,
                priority: Priority::None,
                due_date: None,
                scheduled_date: None,
                recurrence: None,
                done_date: None,
                start_date: None,
                tags: vec![],
                source_file: PathBuf::from("a.md"),
                line_number: 2,
            },
        ];

        let views = sample_views();
        let mut app = test_app(tasks, views);
        app.update_filtered_tasks();

        // Should be sorted by path then line number
        assert_eq!(
            app.tasks[app.filtered_indices[0]].source_file,
            PathBuf::from("a.md")
        );
        assert_eq!(app.tasks[app.filtered_indices[0]].line_number, 1);
        assert_eq!(
            app.tasks[app.filtered_indices[1]].source_file,
            PathBuf::from("a.md")
        );
        assert_eq!(app.tasks[app.filtered_indices[1]].line_number, 2);
        assert_eq!(
            app.tasks[app.filtered_indices[2]].source_file,
            PathBuf::from("b.md")
        );
    }

    #[test]
    fn test_default_view_from_config() {
        let tasks = sample_tasks();
        let views = vec![
            View::new("All Tasks", "", "", ""),
            View::new("Overdue", "due < today", "", ""),
        ];
        let mut config = sample_config();
        config.defaults.view = "Overdue".to_string();
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let app = App::new(config, tasks, views, config_path);
        std::mem::forget(dir);

        assert_eq!(app.current_view.name, "Overdue");
    }

    #[test]
    fn test_default_view_fallback_no_match() {
        let tasks = sample_tasks();
        let views = vec![
            View::new("View1", "not done", "", ""),
            View::new("View2", "done", "", ""),
        ];
        let mut config = sample_config();
        config.defaults.view = "Nonexistent".to_string();
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let app = App::new(config, tasks, views, config_path);
        std::mem::forget(dir);

        assert_eq!(app.current_view.name, "View1"); // falls back to first
    }

    #[test]
    fn test_view_manager_set_default() {
        let tasks = sample_tasks();
        let views = vec![
            View::new("View1", "not done", "", ""),
            View::new("View2", "done", "", ""),
        ];
        let mut app = test_app(tasks, views);
        app.show_view_manager = true;

        // Navigate to View2
        app.handle_view_manager_key(key_event(KeyCode::Down, KeyModifiers::NONE));
        // Set as default
        app.handle_view_manager_key(key_event(KeyCode::Char('s'), KeyModifiers::NONE));

        assert_eq!(app.config.defaults.view, "View2");
    }

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
}
