use crate::task::Task;
use crate::ui::App;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Clear, Paragraph};

/// Returns true if the task matches the given filter text: case-insensitive
/// substring match on the description and on tags. A leading '#' in the
/// filter is ignored for tag matching. Empty/whitespace filter matches all.
pub fn matches_filter(task: &Task, filter: &str) -> bool {
    let needle = filter.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    if task.description.to_lowercase().contains(&needle) {
        return true;
    }
    let tag_needle = needle.trim_start_matches('#');
    task.tags
        .iter()
        .any(|t| t.to_lowercase().contains(tag_needle))
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.filter_textarea.is_none() {
        return;
    }
    match key.code {
        KeyCode::Esc => {
            app.filter_textarea = None;
            app.filter_text.clear();
            app.dirty = true;
        }
        KeyCode::Enter => {
            app.filter_textarea = None;
        }
        _ => {
            if let Some(textarea) = &mut app.filter_textarea {
                textarea.input(key);
                app.filter_text = textarea.lines().join("\n");
                app.dirty = true;
            }
        }
    }
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let Some(textarea) = &app.filter_textarea else {
        return;
    };
    let area = frame.area();
    let bar = Rect::new(0, area.height.saturating_sub(1), area.width, 1);
    frame.render_widget(Clear, bar);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(bar);

    let label = Paragraph::new("/ ").style(Style::default().fg(Color::Yellow));
    frame.render_widget(label, chunks[0]);
    frame.render_widget(textarea, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::sample_tasks;

    #[test]
    fn test_matches_description_case_insensitive() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[0], "BUG"));
        assert!(!matches_filter(&tasks[1], "bug"));
    }

    #[test]
    fn test_matches_tag() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[1], "personal"));
        assert!(matches_filter(&tasks[0], "urgent"));
        assert!(!matches_filter(&tasks[1], "urgent"));
    }

    #[test]
    fn test_matches_tag_with_hash_prefix() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[0], "#work"));
        assert!(matches_filter(&tasks[2], "#WORK"));
    }

    #[test]
    fn test_empty_and_whitespace_matches_all() {
        let tasks = sample_tasks();
        for task in &tasks {
            assert!(matches_filter(task, ""));
            assert!(matches_filter(task, "   "));
        }
    }
}
