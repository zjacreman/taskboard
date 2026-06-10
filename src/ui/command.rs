use crate::ui::App;
use crate::view::View;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.saving_view {
        handle_save_view_key(app, code);
        return;
    }

    match code {
        KeyCode::Esc => {
            app.search_active = false;
            app.search_query.clear();
            app.dirty = true;
        }
        // Alt+Enter submits the query (Ctrl+Enter not reliably detected by terminals)
        KeyCode::Enter if modifiers.contains(KeyModifiers::ALT) => {
            submit_query(app);
        }
        KeyCode::Enter => {
            app.search_query.push('\n');
        }
        KeyCode::Backspace => {
            if app.search_query.is_empty() {
                app.search_active = false;
                app.dirty = true;
            } else {
                app.search_query.pop();
            }
        }
        KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.saving_view = true;
            app.view_name_input.clear();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
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
    let line_count = app.search_query.lines().count().max(1) as u16;
    let popup_height = (line_count + 7).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    // Build the query display with each line separate
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

    // Show the query with cursor
    if app.search_query.is_empty() {
        lines.push(Line::from(Span::styled("█", Style::default().fg(Color::White))));
    } else {
        for (i, line) in app.search_query.lines().enumerate() {
            // Add cursor to the last line
            if i == app.search_query.lines().count() - 1 && !app.search_query.ends_with('\n') {
                lines.push(Line::from(vec![
                    Span::raw(line.to_string()),
                    Span::styled("█", Style::default().fg(Color::White)),
                ]));
            } else {
                lines.push(Line::from(line.to_string()));
            }
        }
        // If query ends with newline, show cursor on new line
        if app.search_query.ends_with('\n') {
            lines.push(Line::from(Span::styled("█", Style::default().fg(Color::White))));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(": newline | "),
        Span::styled("Alt+Enter", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(": apply | "),
        Span::styled("Ctrl+S", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(": save as view | "),
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
    let popup_height = 7.min(area.height - 4);
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
            format!("Query: {}", app.search_query.lines().next().unwrap_or("")),
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
