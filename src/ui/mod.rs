pub mod task_list;
pub mod modal;
pub mod command;
pub mod theme;

use crate::config::Config;
use crate::task::Task;
use crate::view::View;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

pub struct App {
    pub tasks: Vec<Task>,
    pub filtered_tasks: Vec<Task>,
    pub selected_index: usize,
    pub views: Vec<View>,
    pub current_view: View,
    pub config: Config,
    pub should_quit: bool,
    pub show_help: bool,
}

impl App {
    pub fn new(config: Config, tasks: Vec<Task>, views: Vec<View>) -> Self {
        let current_view = views.first().cloned().unwrap_or_default();
        Self {
            tasks,
            filtered_tasks: Vec::new(),
            selected_index: 0,
            views,
            current_view,
            config,
            should_quit: false,
            show_help: false,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        loop {
            self.update_filtered_tasks();
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => self.selected_index = 0,
            KeyCode::Char('G') => self.selected_index = self.filtered_tasks.len().saturating_sub(1),
            KeyCode::Char('x') => self.toggle_done(),
            KeyCode::Char('p') => self.cycle_priority(),
            KeyCode::Char('d') => self.set_due_date_today(),
            KeyCode::Char('D') => self.set_due_date_tomorrow(),
            KeyCode::Char('s') => self.set_scheduled_today(),
            KeyCode::Char('S') => self.set_scheduled_tomorrow(),
            KeyCode::Char('b') => self.bump_scheduled(),
            _ => {}
        }
    }

    fn move_down(&mut self) {
        if !self.filtered_tasks.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_tasks.len();
        }
    }

    fn move_up(&mut self) {
        if !self.filtered_tasks.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_tasks.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    fn toggle_done(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.status.cycle();
        }
    }

    fn cycle_priority(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.priority.cycle();
        }
    }

    fn set_due_date_today(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.due_date = Some(chrono::Local::now().date_naive());
        }
    }

    fn set_due_date_tomorrow(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.due_date = Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
        }
    }

    fn set_scheduled_today(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.scheduled_date = Some(chrono::Local::now().date_naive());
        }
    }

    fn set_scheduled_tomorrow(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            task.scheduled_date = Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
        }
    }

    fn bump_scheduled(&mut self) {
        if let Some(task) = self.filtered_tasks.get_mut(self.selected_index) {
            let date = task.scheduled_date.unwrap_or_else(|| chrono::Local::now().date_naive());
            task.scheduled_date = Some(date + chrono::Duration::days(1));
        }
    }

    pub fn update_filtered_tasks(&mut self) {
        self.filtered_tasks = crate::task::query::execute_query(
            &self.current_view.query,
            &self.tasks,
        ).unwrap_or_default();
    }

    fn draw(&self, frame: &mut ratatui::Frame) {
        task_list::draw(frame, self);
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, WorkspaceConfig, DefaultsConfig, ThemeConfig};
    use crate::task::{Task, TaskStatus, Priority};
    use crate::view::View;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    use super::App;

    fn sample_config() -> Config {
        Config {
            workspace: WorkspaceConfig { path: ".".to_string() },
            defaults: DefaultsConfig::default(),
            theme: ThemeConfig::default(),
        }
    }

    fn sample_tasks() -> Vec<Task> {
        vec![
            Task {
                description: "Buy groceries".to_string(),
                status: TaskStatus::Todo,
                priority: Priority::None,
                due_date: Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
                scheduled_date: None,
                recurrence: None,
                done_date: None,
                start_date: None,
                tags: vec!["personal".to_string()],
                source_file: PathBuf::from("tasks.md"),
                line_number: 1,
            },
            Task {
                description: "Review PR".to_string(),
                status: TaskStatus::Done,
                priority: Priority::High,
                due_date: Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
                scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 6, 9).unwrap()),
                recurrence: None,
                done_date: Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
                start_date: None,
                tags: vec!["work".to_string()],
                source_file: PathBuf::from("work.md"),
                line_number: 5,
            },
            Task {
                description: "Fix bug".to_string(),
                status: TaskStatus::Todo,
                priority: Priority::Medium,
                due_date: None,
                scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()),
                recurrence: Some("every week".to_string()),
                done_date: None,
                start_date: None,
                tags: vec!["work".to_string(), "urgent".to_string()],
                source_file: PathBuf::from("bugs.md"),
                line_number: 10,
            },
        ]
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

        assert_eq!(app.filtered_tasks[0].status, TaskStatus::Todo);
        app.toggle_done();
        assert_eq!(app.filtered_tasks[0].status, TaskStatus::Done);
        app.toggle_done();
        assert_eq!(app.filtered_tasks[0].status, TaskStatus::Todo);
    }

    #[test]
    fn test_cycle_priority() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.filtered_tasks[0].priority, Priority::None);
        app.cycle_priority();
        assert_eq!(app.filtered_tasks[0].priority, Priority::High);
    }

    #[test]
    fn test_set_due_date_today() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        // Select task with no due date
        app.selected_index = 2;
        app.set_due_date_today();
        assert_eq!(app.filtered_tasks[2].due_date, Some(chrono::Local::now().date_naive()));
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
        assert_eq!(app.filtered_tasks[0].due_date, Some(tomorrow));
    }

    #[test]
    fn test_set_scheduled_today() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        app.set_scheduled_today();
        assert_eq!(app.filtered_tasks[0].scheduled_date, Some(chrono::Local::now().date_naive()));
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
        assert_eq!(app.filtered_tasks[0].scheduled_date, Some(tomorrow));
    }

    #[test]
    fn test_bump_scheduled() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        // Task at index 2 has scheduled_date = 2026-06-12
        app.selected_index = 2;
        let original = app.filtered_tasks[2].scheduled_date.unwrap();
        app.bump_scheduled();
        assert_eq!(app.filtered_tasks[2].scheduled_date, Some(original + chrono::Duration::days(1)));
    }

    #[test]
    fn test_bump_scheduled_no_existing() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        // Task at index 0 has no scheduled_date
        app.selected_index = 0;
        app.bump_scheduled();
        let today = chrono::Local::now().date_naive();
        assert_eq!(app.filtered_tasks[0].scheduled_date, Some(today + chrono::Duration::days(1)));
    }

    #[test]
    fn test_update_filtered_tasks() {
        let config = sample_config();
        let tasks = sample_tasks();
        let mut views = sample_views();
        views[0].query = "not done".to_string();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.filtered_tasks.len(), 2);
    }

    #[test]
    fn test_update_filtered_tasks_empty_query() {
        let config = sample_config();
        let tasks = sample_tasks();
        let views = sample_views();
        let mut app = App::new(config, tasks, views);
        app.update_filtered_tasks();

        assert_eq!(app.filtered_tasks.len(), 3);
    }

    #[test]
    fn test_default_view_used_when_empty() {
        let config = sample_config();
        let tasks = sample_tasks();
        let app = App::new(config, tasks, vec![]);

        assert_eq!(app.current_view.name, "All Tasks");
    }
}
