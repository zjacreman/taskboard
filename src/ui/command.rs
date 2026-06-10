use crate::ui::App;
use crate::view::View;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

fn query_lines(s: &str) -> Vec<String> {
    s.split('\n').map(|s| s.to_string()).collect()
}

fn query_line_refs(s: &str) -> Vec<&str> {
    s.split('\n').collect()
}

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.saving_view {
        handle_save_view_key(app, code);
        return;
    }

    match code {
        KeyCode::Esc => {
            app.search_active = false;
            app.search_query.clear();
            app.search_cursor_row = 0;
            app.search_cursor_col = 0;
            app.dirty = true;
        }
        KeyCode::Enter if modifiers.contains(KeyModifiers::ALT) => {
            submit_query(app);
        }
        KeyCode::Enter => {
            let lines = query_lines(&app.search_query);
            let row = app.search_cursor_row.min(lines.len().saturating_sub(1));
            let line = lines.get(row).cloned().unwrap_or_default();
            let col = app.search_cursor_col.min(line.len());

            let before = &line[..col];
            let after = &line[col..];

            let mut new_lines: Vec<String> = lines[..row].to_vec();
            new_lines.push(before.to_string());
            new_lines.push(after.to_string());
            new_lines.extend_from_slice(&lines[row + 1..]);

            app.search_query = new_lines.join("\n");
            app.search_cursor_row += 1;
            app.search_cursor_col = 0;
        }
        KeyCode::Backspace => {
            if app.search_query.is_empty() {
                app.search_active = false;
                app.dirty = true;
                return;
            }

            let lines = query_lines(&app.search_query);
            let row = app.search_cursor_row.min(lines.len().saturating_sub(1));
            let line = lines.get(row).cloned().unwrap_or_default();
            let col = app.search_cursor_col.min(line.len());

            if col > 0 {
                let mut new_line = line[..col - 1].to_string();
                new_line.push_str(&line[col..]);
                let mut new_lines = lines;
                new_lines[row] = new_line;
                app.search_query = new_lines.join("\n");
                app.search_cursor_col -= 1;
            } else if row > 0 {
                let prev_line = &lines[row - 1];
                let new_col = prev_line.len();
                let mut new_lines: Vec<String> = lines[..row - 1].to_vec();
                new_lines.push(format!("{}{}", prev_line, line));
                new_lines.extend_from_slice(&lines[row + 1..]);
                app.search_query = new_lines.join("\n");
                app.search_cursor_row -= 1;
                app.search_cursor_col = new_col;
            }
        }
        KeyCode::Delete => {
            let lines = query_lines(&app.search_query);
            let row = app.search_cursor_row.min(lines.len().saturating_sub(1));
            let line = lines.get(row).cloned().unwrap_or_default();
            let col = app.search_cursor_col.min(line.len());

            if col < line.len() {
                let mut new_line = line[..col].to_string();
                new_line.push_str(&line[col + 1..]);
                let mut new_lines = lines;
                new_lines[row] = new_line;
                app.search_query = new_lines.join("\n");
            } else if row + 1 < lines.len() {
                let mut new_lines: Vec<String> = lines[..row].to_vec();
                new_lines.push(format!("{}{}", line, lines[row + 1]));
                new_lines.extend_from_slice(&lines[row + 2..]);
                app.search_query = new_lines.join("\n");
            }
        }
        KeyCode::Left => {
            if app.search_cursor_col > 0 {
                app.search_cursor_col -= 1;
            } else if app.search_cursor_row > 0 {
                app.search_cursor_row -= 1;
                let lines = query_line_refs(&app.search_query);
                app.search_cursor_col = lines.get(app.search_cursor_row).map(|l| l.len()).unwrap_or(0);
            }
        }
        KeyCode::Right => {
            let lines = query_lines(&app.search_query);
            let row = app.search_cursor_row.min(lines.len().saturating_sub(1));
            let line_len = lines.get(row).map(|l| l.len()).unwrap_or(0);

            if app.search_cursor_col < line_len {
                app.search_cursor_col += 1;
            } else if row + 1 < lines.len() {
                app.search_cursor_row += 1;
                app.search_cursor_col = 0;
            }
        }
        KeyCode::Up if app.search_cursor_row > 0 => {
            app.search_cursor_row -= 1;
            let lines = query_line_refs(&app.search_query);
            let line_len = lines.get(app.search_cursor_row).map(|l| l.len()).unwrap_or(0);
            app.search_cursor_col = app.search_cursor_col.min(line_len);
        }
        KeyCode::Down => {
            let lines = query_line_refs(&app.search_query);
            if app.search_cursor_row + 1 < lines.len() {
                app.search_cursor_row += 1;
                let line_len = lines.get(app.search_cursor_row).map(|l| l.len()).unwrap_or(0);
                app.search_cursor_col = app.search_cursor_col.min(line_len);
            }
        }
        KeyCode::Home => {
            app.search_cursor_col = 0;
        }
        KeyCode::End => {
            let lines = query_line_refs(&app.search_query);
            app.search_cursor_col = lines.get(app.search_cursor_row).map(|l| l.len()).unwrap_or(0);
        }
        KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.saving_view = true;
            app.view_name_input.clear();
        }
        KeyCode::Char(c) => {
            let lines = query_lines(&app.search_query);
            let row = app.search_cursor_row.min(lines.len().saturating_sub(1));
            let line = lines.get(row).cloned().unwrap_or_default();
            let col = app.search_cursor_col.min(line.len());

            let mut new_line = line[..col].to_string();
            new_line.push(c);
            new_line.push_str(&line[col..]);

            let mut new_lines = lines;
            if new_lines.is_empty() {
                new_lines.push(new_line);
            } else {
                new_lines[row] = new_line;
            }
            app.search_query = new_lines.join("\n");
            app.search_cursor_col += 1;
        }
        _ => {}
    }
}

fn handle_save_view_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.saving_view = false;
            app.view_name_input.clear();
        }
        KeyCode::Enter => {
            let name = if app.view_name_input.is_empty() {
                "Untitled".to_string()
            } else {
                app.view_name_input.clone()
            };

            let view = View::new(
                &name,
                &app.search_query,
                &app.current_view.sort_by,
                &app.current_view.group_by,
            );

            app.views.push(view);
            app.save_views();

            app.saving_view = false;
            app.view_name_input.clear();
            app.search_active = false;
            app.dirty = true;
        }
        KeyCode::Backspace => {
            app.view_name_input.pop();
        }
        KeyCode::Char(c) => {
            app.view_name_input.push(c);
        }
        _ => {}
    }
}

fn submit_query(app: &mut App) {
    app.current_view.query = app.search_query.clone();
    app.search_active = false;
    app.dirty = true;
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    if app.saving_view {
        draw_save_view(frame, app);
        return;
    }

    let area = frame.area();
    let popup_width = 70.min(area.width - 4);
    let line_count = query_line_refs(&app.search_query).len().max(1) as u16;
    let popup_height = (line_count + 12).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "One filter per line. Example:",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  not done\ndue before tomorrow\nsort by priority",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    let query_lines = query_lines(&app.search_query);

    for (i, line) in query_lines.iter().enumerate() {
        if i == app.search_cursor_row {
            let col = app.search_cursor_col.min(line.len());
            let before = &line[..col];
            let after = &line[col..];
            lines.push(Line::from(vec![
                Span::raw(before.to_string()),
                Span::styled("█", Style::default().fg(Color::White)),
                Span::raw(after.to_string()),
            ]));
        } else {
            lines.push(Line::from(line.to_string()));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(": newline"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Alt+Enter", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(": apply query"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+S", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(": save as view"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Esc", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(": cancel"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search / Query")
                .style(Style::default().bg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, popup_area);
}

fn draw_save_view(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    let popup_width = 50.min(area.width - 4);
    let popup_height = 8.min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let content = vec![
        Line::from(Span::styled(
            "Save as view — enter name:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw(&app.view_name_input),
            Span::styled("█", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("Query: {}", query_line_refs(&app.search_query).first().unwrap_or(&"")),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Enter: save | Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Save View")
                .style(Style::default().bg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, popup_area);
}
