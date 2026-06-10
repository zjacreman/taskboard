use crate::ui::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_task_list(frame, app, chunks[0]);
    draw_status_bar(frame, app, chunks[1]);
}

fn draw_task_list(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let width = area.width as usize;

    let items: Vec<ListItem> = app
        .filtered_tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let status = if task.status == crate::task::TaskStatus::Done {
                "[x]"
            } else {
                "[ ]"
            };

            let priority = task.priority.to_emoji();
            let priority_str = if priority.is_empty() {
                String::new()
            } else {
                format!("{} ", priority)
            };

            let due = task
                .due_date
                .map(|d| {
                    if width < 80 {
                        format!(" 📅{}", d.format("%m-%d"))
                    } else {
                        format!(" 📅 {}", d.format("%Y-%m-%d"))
                    }
                })
                .unwrap_or_default();

            let scheduled = task
                .scheduled_date
                .map(|d| {
                    if width < 80 {
                        format!(" 🛫{}", d.format("%m-%d"))
                    } else {
                        format!(" 🛫 {}", d.format("%Y-%m-%d"))
                    }
                })
                .unwrap_or_default();

            let source = if width >= 120 {
                format!(" {}", task.source_file.display())
            } else {
                String::new()
            };

            let line = if i == app.selected_index {
                Line::from(vec![
                    Span::styled(
                        format!("{} {}{}{}{}{}", status, priority_str, task.description, due, scheduled, source),
                        Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(status, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::raw(priority_str),
                    Span::raw(task.description.clone()),
                    Span::styled(format!("{}{}{}", due, scheduled, source), Style::default().fg(Color::DarkGray)),
                ])
            };

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("TaskBoard - {}", app.current_view.name)),
    );

    frame.render_widget(list, area);
}

fn draw_status_bar(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let total = app.filtered_tasks.len();
    let done = app
        .filtered_tasks
        .iter()
        .filter(|t| t.status == crate::task::TaskStatus::Done)
        .count();
    let open = total - done;

    let status = format!(
        "{} tasks ({} done, {} open) | {} | q:quit / /:search / ?:help",
        total, done, open, app.current_view.name
    );

    let paragraph = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}
