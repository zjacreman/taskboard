use crate::ui::App;
use crate::task::TaskStatus;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.show_modal = false,
        KeyCode::Enter => {
            app.show_modal = false;
        }
        KeyCode::Tab => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].status.cycle();
                app.dirty = true;
            }
        }
        KeyCode::Char('x') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].status.cycle();
                app.dirty = true;
            }
        }
        KeyCode::Char('p') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].priority.cycle();
                app.dirty = true;
            }
        }
        _ => {}
    }
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    let popup_width = 60.min(area.width - 4);
    let popup_height = 15.min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let Some(idx) = app.selected_task_index() else {
        return;
    };
    let task = &app.tasks[idx];

    let status_str = match task.status {
        TaskStatus::Todo => "[ ]",
        TaskStatus::Done => "[x]",
    };

    let priority_str = task.priority.to_emoji();
    let due_str = task.due_date
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "none".to_string());
    let scheduled_str = task.scheduled_date
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "none".to_string());
    let recurrence_str = task.recurrence
        .clone()
        .unwrap_or_else(|| "none".to_string());

    let content = vec![
        Line::from(vec![
            Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
            {
                let rel = app.workspace_path.as_ref()
                    .and_then(|wp| task.source_file.strip_prefix(wp).ok())
                    .unwrap_or(&task.source_file);
                Span::raw(format!("{}:{}", rel.display(), task.line_number))
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Description: ", Style::default().fg(Color::DarkGray)),
            Span::raw(task.description.clone()),
        ]),
        Line::from(vec![
            Span::styled("Status:     ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} (x to toggle)", status_str)),
        ]),
        Line::from(vec![
            Span::styled("Priority:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} (p to cycle)", priority_str)),
        ]),
        Line::from(vec![
            Span::styled("Due:        ", Style::default().fg(Color::DarkGray)),
            Span::raw(due_str),
        ]),
        Line::from(vec![
            Span::styled("Scheduled:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(scheduled_str),
        ]),
        Line::from(vec![
            Span::styled("Recurrence: ", Style::default().fg(Color::DarkGray)),
            Span::raw(recurrence_str),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Enter: save | Esc: cancel | Tab/x: toggle status | p: cycle priority",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Edit Task")
                .style(Style::default().bg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, popup_area);
}
