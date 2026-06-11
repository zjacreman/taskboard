use crate::ui::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

const COMPACT_WIDTH: usize = 80;
const WIDE_WIDTH: usize = 120;
const PATH_INDENT: &str = "    ";

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

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.len() <= max_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.len() > max_width {
                for chunk in word.as_bytes().chunks(max_width) {
                    lines.push(String::from_utf8_lossy(chunk).to_string());
                }
            } else {
                current_line = word.to_string();
            }
        } else if current_line.len() + 1 + word.len() > max_width {
            lines.push(current_line);
            if word.len() > max_width {
                for chunk in word.as_bytes().chunks(max_width) {
                    lines.push(String::from_utf8_lossy(chunk).to_string());
                }
                current_line = String::new();
            } else {
                current_line = word.to_string();
            }
        } else {
            current_line.push(' ');
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn draw_task_list(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let width = area.width as usize;
    let inner_width = width.saturating_sub(2); // Account for borders

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

            let (source, show_path_below) = if width >= WIDE_WIDTH {
                let rel = app.workspace_path.as_ref()
                    .and_then(|wp| task.source_file.strip_prefix(wp).ok())
                    .unwrap_or(&task.source_file);
                (format!(" {}", rel.display()), false)
            } else {
                (String::new(), true)
            };

            let path_display = if show_path_below {
                let rel = app.workspace_path.as_ref()
                    .and_then(|wp| task.source_file.strip_prefix(wp).ok())
                    .unwrap_or(&task.source_file);
                rel.display().to_string()
            } else {
                String::new()
            };

            let prefix_len = status.len() + 1 + priority_str.len();
            let metadata_len = due.len() + scheduled.len() + source.len();
            let desc_max_width = inner_width.saturating_sub(prefix_len + metadata_len).max(20);

            let desc_lines = wrap_text(&task.description, desc_max_width);

            let mut lines: Vec<Line> = Vec::new();

            if i == app.selected_index {
                let first_desc = desc_lines.first().cloned().unwrap_or_default();
                let main_line = format!("{} {}{}{}{}{}", status, priority_str, first_desc, due, scheduled, source);
                lines.push(Line::from(vec![
                    Span::styled(
                        main_line,
                        Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ]));
                for desc_line in desc_lines.iter().skip(1) {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("{}{}", " ".repeat(prefix_len), desc_line),
                            Style::default().fg(Color::Black).bg(Color::White),
                        ),
                    ]));
                }
                if show_path_below && !path_display.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("{}{}", PATH_INDENT, path_display),
                            Style::default().fg(Color::Black).bg(Color::White),
                        ),
                    ]));
                }
            } else {
                let first_desc = desc_lines.first().cloned().unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::styled(status, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::raw(priority_str),
                    Span::raw(first_desc),
                    Span::styled(format!("{}{}{}", due, scheduled, source), Style::default().fg(Color::DarkGray)),
                ]));
                for desc_line in desc_lines.iter().skip(1) {
                    lines.push(Line::from(vec![
                        Span::raw(" ".repeat(prefix_len)),
                        Span::raw(desc_line.clone()),
                    ]));
                }
                if show_path_below && !path_display.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("{}{}", PATH_INDENT, path_display),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
            }

            ListItem::new(lines)
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

    let status = if let Some(msg) = &app.status_message {
        format!("{} | q:quit / /:search / ?:help", msg)
    } else {
        format!(
            "{} tasks ({} done, {} open) | {} | q:quit / /:search / ?:help",
            total, done, open, app.current_view.name
        )
    };

    let style = if app.status_message.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let paragraph = Paragraph::new(status).style(style);
    frame.render_widget(paragraph, area);
}
