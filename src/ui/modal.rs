use crate::task::{Priority, TaskStatus};
use crate::ui::App;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tui_textarea::{CursorMove, TextArea};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EditField {
    Description,
    Status,
    Priority,
    DueDate,
    ScheduledDate,
    Recurrence,
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let code = key.code;
    if app.task_edit.is_some() {
        handle_field_edit(app, key);
        return;
    }

    match code {
        KeyCode::Esc => app.show_modal = false,
        KeyCode::Enter => {
            app.show_modal = false;
            app.dirty = true;
        }
        KeyCode::Tab | KeyCode::Down => {
            app.task_edit_field = next_field(app.task_edit_field);
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.task_edit_field = prev_field(app.task_edit_field);
        }
        KeyCode::Char('e') | KeyCode::Char(' ') => {
            start_field_edit(app);
        }
        KeyCode::Char('x') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].status.cycle();
                match app.tasks[idx].status {
                    TaskStatus::Done => {
                        app.tasks[idx].done_date = Some(chrono::Local::now().date_naive());
                    }
                    TaskStatus::Todo => {
                        app.tasks[idx].done_date = None;
                    }
                }
                app.persist_task(idx);
                app.dirty = true;
            }
        }
        KeyCode::Char('p') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].priority.cycle();
                app.persist_task(idx);
                app.dirty = true;
            }
        }
        KeyCode::Char('d') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].due_date = Some(chrono::Local::now().date_naive());
                app.persist_task(idx);
                app.dirty = true;
            }
        }
        KeyCode::Char('D') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].due_date =
                    Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
                app.persist_task(idx);
                app.dirty = true;
            }
        }
        KeyCode::Char('s') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].scheduled_date = Some(chrono::Local::now().date_naive());
                app.persist_task(idx);
                app.dirty = true;
            }
        }
        KeyCode::Char('S') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].scheduled_date =
                    Some(chrono::Local::now().date_naive() + chrono::Duration::days(1));
                app.persist_task(idx);
                app.dirty = true;
            }
        }
        KeyCode::Char('b') => {
            if let Some(idx) = app.selected_task_index() {
                let date = app.tasks[idx]
                    .scheduled_date
                    .unwrap_or_else(|| chrono::Local::now().date_naive());
                app.tasks[idx].scheduled_date = Some(date + chrono::Duration::days(1));
                app.persist_task(idx);
                app.dirty = true;
            }
        }
        KeyCode::Char('r') => {
            if let Some(idx) = app.selected_task_index() {
                app.tasks[idx].recurrence = None;
                app.persist_task(idx);
                app.dirty = true;
            }
        }
        _ => {}
    }
}

fn next_field(f: EditField) -> EditField {
    match f {
        EditField::Description => EditField::Status,
        EditField::Status => EditField::Priority,
        EditField::Priority => EditField::DueDate,
        EditField::DueDate => EditField::ScheduledDate,
        EditField::ScheduledDate => EditField::Recurrence,
        EditField::Recurrence => EditField::Description,
    }
}

fn prev_field(f: EditField) -> EditField {
    match f {
        EditField::Description => EditField::Recurrence,
        EditField::Status => EditField::Description,
        EditField::Priority => EditField::Status,
        EditField::DueDate => EditField::Priority,
        EditField::ScheduledDate => EditField::DueDate,
        EditField::Recurrence => EditField::ScheduledDate,
    }
}

fn start_field_edit(app: &mut App) {
    let Some(idx) = app.selected_task_index() else {
        return;
    };
    match app.task_edit_field {
        EditField::Priority => {
            app.tasks[idx].priority.cycle();
            app.persist_task(idx);
            app.dirty = true;
            return;
        }
        _ => {}
    }
    let text = match app.task_edit_field {
        EditField::Description => app.tasks[idx].description.clone(),
        EditField::Status => match app.tasks[idx].status {
            TaskStatus::Todo => "todo".to_string(),
            TaskStatus::Done => "done".to_string(),
        },
        EditField::Priority => unreachable!(),
        EditField::DueDate => app.tasks[idx]
            .due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        EditField::ScheduledDate => app.tasks[idx]
            .scheduled_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        EditField::Recurrence => app.tasks[idx].recurrence.clone().unwrap_or_default(),
    };
    let mut textarea = TextArea::new(vec![text]);
    textarea.set_cursor_line_style(ratatui::style::Style::default());
    textarea.move_cursor(CursorMove::End);
    app.task_edit = Some(textarea);
}

fn handle_field_edit(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.task_edit = None;
        }
        KeyCode::Enter => {
            apply_field_edit(app);
            app.task_edit = None;
        }
        _ => {
            if let Some(textarea) = &mut app.task_edit {
                textarea.input(key);
            }
        }
    }
}

fn apply_field_edit(app: &mut App) {
    let Some(textarea) = &app.task_edit else {
        return;
    };
    let text = textarea.lines()[0].clone();
    let Some(idx) = app.selected_task_index() else {
        return;
    };
    match app.task_edit_field {
        EditField::Description => {
            app.tasks[idx].description = text;
        }
        EditField::Status => {
            app.tasks[idx].status = match text.to_lowercase().as_str() {
                "done" | "x" | "[x]" => TaskStatus::Done,
                _ => TaskStatus::Todo,
            };
        }
        EditField::Priority => {
            app.tasks[idx].priority = match text.to_lowercase().as_str() {
                "highest" | "🔺" => Priority::Highest,
                "high" | "⏫" => Priority::High,
                "medium" | "🔼" => Priority::Medium,
                "low" | "🔽" => Priority::Low,
                "lowest" | "⏬" => Priority::Lowest,
                _ => Priority::None,
            };
        }
        EditField::DueDate => {
            app.tasks[idx].due_date = parse_date_input(&text);
        }
        EditField::ScheduledDate => {
            app.tasks[idx].scheduled_date = parse_date_input(&text);
        }
        EditField::Recurrence => {
            if text.is_empty() {
                app.tasks[idx].recurrence = None;
            } else {
                app.tasks[idx].recurrence = Some(text);
            }
        }
    }
    app.persist_task(idx);
    app.dirty = true;
}

fn parse_date_input(s: &str) -> Option<chrono::NaiveDate> {
    let s = s.trim();
    if s.is_empty() || s == "none" || s == "clear" {
        return None;
    }
    let today = chrono::Local::now().date_naive();
    match s {
        "today" => Some(today),
        "tomorrow" => Some(today + chrono::Duration::days(1)),
        "yesterday" => Some(today - chrono::Duration::days(1)),
        _ => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok(),
    }
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    let popup_width = 65.min(area.width - 4);
    // 1 source + 1 empty + 6 fields + 1 empty + 3 or 4 help lines + 2 border
    let editing = app.task_edit.is_some();
    let (edit_text, cursor_col) = if let Some(textarea) = &app.task_edit {
        (textarea.lines()[0].as_str(), textarea.cursor().1)
    } else {
        ("", 0)
    };
    let help_lines = if editing { 4 } else { 3 };
    let popup_height = (10 + help_lines + 2).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let Some(idx) = app.selected_task_index() else {
        return;
    };
    let task = &app.tasks[idx];

    let selected = app.task_edit_field;

    let mut lines: Vec<Line> = Vec::new();

    let rel = app
        .workspace_path
        .as_ref()
        .and_then(|wp| task.source_file.strip_prefix(wp).ok())
        .unwrap_or(&task.source_file);
    lines.push(Line::from(Span::styled(
        format!("{}:{}", rel.display(), task.line_number),
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(""));

    lines.push(field_line(
        "Description",
        &task.description,
        selected == EditField::Description,
        editing,
        edit_text,
        cursor_col,
    ));
    lines.push(field_line(
        "Status",
        status_display(task.status),
        selected == EditField::Status,
        editing,
        edit_text,
        cursor_col,
    ));
    lines.push(field_line(
        "Priority",
        priority_display(task.priority),
        selected == EditField::Priority,
        editing,
        edit_text,
        cursor_col,
    ));
    lines.push(field_line(
        "Due Date",
        &date_display(task.due_date),
        selected == EditField::DueDate,
        editing,
        edit_text,
        cursor_col,
    ));
    lines.push(field_line(
        "Scheduled",
        &date_display(task.scheduled_date),
        selected == EditField::ScheduledDate,
        editing,
        edit_text,
        cursor_col,
    ));
    lines.push(field_line(
        "Recurrence",
        task.recurrence.as_deref().unwrap_or("none"),
        selected == EditField::Recurrence,
        editing,
        edit_text,
        cursor_col,
    ));

    lines.push(Line::from(""));
    if editing {
        lines.push(Line::from(Span::styled(
            "Enter: save | Esc: cancel",
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(Span::styled(
            "Date: today/tomorrow/YYYY-MM-DD/none",
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(Span::styled(
            "Status: todo/done | Priority: lowest..highest/none",
            Style::default().fg(Color::Gray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Tab/↑↓: navigate | e/Space: edit field | Enter/Esc: close",
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(Span::styled(
            "x: toggle status | d/D: due today/tmr | s/S: sched today/tmr",
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(Span::styled(
            "b: bump sched | p: cycle priority | r: clear recurrence",
            Style::default().fg(Color::Gray),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Edit Task")
                .style(Style::default().bg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, popup_area);
}

fn field_line<'a>(
    label: &str,
    value: &str,
    selected: bool,
    editing: bool,
    edit_text: &str,
    cursor_col: usize,
) -> Line<'a> {
    let label_style = Style::default().fg(Color::Gray);
    let marker = if selected { "▸ " } else { "  " };

    if selected && editing {
        let before = &edit_text[..cursor_col.min(edit_text.len())];
        let after = &edit_text[cursor_col.min(edit_text.len())..];
        Line::from(vec![
            Span::raw(marker),
            Span::styled(format!("{:12}", label), label_style),
            Span::styled("│ ", Style::default().fg(Color::Cyan)),
            Span::raw(before.to_string()),
            Span::styled("█", Style::default().fg(Color::White)),
            Span::raw(after.to_string()),
        ])
    } else if selected {
        Line::from(vec![
            Span::styled(marker, Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:12}", label), label_style),
            Span::styled("│ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                value.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (e to edit)", Style::default().fg(Color::Gray)),
        ])
    } else {
        Line::from(vec![
            Span::raw(marker),
            Span::styled(format!("{:12}", label), label_style),
            Span::raw("  "),
            Span::raw(value.to_string()),
        ])
    }
}

fn status_display(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Todo => "todo",
        TaskStatus::Done => "done",
    }
}

fn priority_display(p: Priority) -> &'static str {
    match p {
        Priority::Highest => "highest",
        Priority::High => "high",
        Priority::Medium => "medium",
        Priority::Low => "low",
        Priority::Lowest => "lowest",
        Priority::None => "none",
    }
}

fn date_display(d: Option<chrono::NaiveDate>) -> String {
    d.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "none".to_string())
}
