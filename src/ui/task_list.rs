use crate::ui::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

const COMPACT_WIDTH: usize = 80;
const WIDE_WIDTH: usize = 120;

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

fn format_due_date(date: chrono::NaiveDate, width: usize) -> String {
    if width < COMPACT_WIDTH {
        format!(" 📅{}", date.format("%m-%d"))
    } else {
        format!(" 📅 {}", date.format("%Y-%m-%d"))
    }
}

fn format_scheduled_date(date: chrono::NaiveDate, width: usize) -> String {
    if width < COMPACT_WIDTH {
        format!(" 🛫{}", date.format("%m-%d"))
    } else {
        format!(" 🛫 {}", date.format("%Y-%m-%d"))
    }
}

fn draw_task_list(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let width = area.width as usize;

    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(i, &task_idx)| {
            let task = &app.tasks[task_idx];

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

            let due = task.due_date
                .map(|d| format_due_date(d, width))
                .unwrap_or_default();

            let scheduled = task.scheduled_date
                .map(|d| format_scheduled_date(d, width))
                .unwrap_or_default();

            let source = if width >= WIDE_WIDTH {
                let rel = app.workspace_path.as_ref()
                    .and_then(|wp| task.source_file.strip_prefix(wp).ok())
                    .unwrap_or(&task.source_file);
                format!(" {}", rel.display())
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

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("TaskBoard - {}", app.current_view.name)),
        );

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_status_bar(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let total = app.filtered_indices.len();
    let done = app
        .filtered_indices
        .iter()
        .filter(|&&idx| app.tasks[idx].status == crate::task::TaskStatus::Done)
        .count();
    let open = total - done;

    let status = format!(
        "{} tasks ({} done, {} open) | {} | q:quit / /:search / ?:help",
        total, done, open, app.current_view.name
    );

    let paragraph = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}
