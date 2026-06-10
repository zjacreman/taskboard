use crate::ui::App;
use crate::view::View;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tui_textarea::TextArea;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.save_view_edit.is_some() {
        handle_save_view_key(app, key);
        return;
    }

    let Some(textarea) = &mut app.search_textarea else { return };

    match key.code {
        KeyCode::Esc => {
            app.search_textarea = None;
            app.dirty = true;
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
            submit_query(app);
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut save_edit = TextArea::new(vec![String::new()]);
            save_edit.set_cursor_line_style(ratatui::style::Style::default());
            app.save_view_edit = Some(save_edit);
        }
        _ => {
            textarea.input(key);
        }
    }
}

fn handle_save_view_key(app: &mut App, key: KeyEvent) {
    let Some(textarea) = &mut app.save_view_edit else { return };
    match key.code {
        KeyCode::Esc => {
            app.save_view_edit = None;
        }
        KeyCode::Enter => {
            let name = if textarea.lines()[0].is_empty() {
                "Untitled".to_string()
            } else {
                textarea.lines()[0].clone()
            };

            let view = View::new(
                &name,
                &app.current_view.query,
                &app.current_view.sort_by,
                &app.current_view.group_by,
            );

            app.views.push(view);
            app.save_views();

            app.save_view_edit = None;
            app.search_textarea = None;
            app.dirty = true;
        }
        _ => {
            textarea.input(key);
        }
    }
}

fn submit_query(app: &mut App) {
    let Some(textarea) = &app.search_textarea else { return };
    app.current_view.query = textarea.lines().join("\n");
    app.search_textarea = None;
    app.dirty = true;
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    if app.save_view_edit.is_some() {
        draw_save_view(frame, app);
        return;
    }

    let Some(textarea) = &app.search_textarea else { return };

    let area = frame.area();
    let popup_width = 70.min(area.width - 4);
    let line_count = textarea.lines().len().max(1) as u16;
    let popup_height = (line_count + 12).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Search / Query")
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, popup_area);

    let inner = popup_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(4),
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(5),
        ])
        .split(inner);

    let instructions = Paragraph::new(vec![
        Line::from(Span::styled("One filter per line. Example:", Style::default().fg(Color::Gray))),
        Line::from(Span::styled("  not done\ndue before tomorrow\nsort by priority", Style::default().fg(Color::Gray))),
    ]);
    frame.render_widget(instructions, chunks[0]);

    frame.render_widget(textarea, chunks[1]);

    let help = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(": newline"),
        ]),
        Line::from(vec![
            Span::styled("Alt+Enter", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(": apply query"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+S", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(": save as view"),
        ]),
        Line::from(vec![
            Span::styled("Esc", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(": cancel"),
        ]),
    ]);
    frame.render_widget(help, chunks[2]);
}

fn draw_save_view(frame: &mut ratatui::Frame, app: &App) {
    let Some(textarea) = &app.save_view_edit else { return };

    let area = frame.area();
    let popup_width = 50.min(area.width - 4);
    let popup_height = 8.min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Save View")
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, popup_area);

    let inner = popup_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(inner);

    let title = Paragraph::new(Span::styled(
        "Save as view — enter name:",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(title, chunks[0]);

    frame.render_widget(textarea, chunks[1]);

    let query_preview = Paragraph::new(Span::styled(
        format!("Query: {}", app.current_view.query.lines().next().unwrap_or("")),
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(query_preview, chunks[3]);

    let help = Paragraph::new(Span::styled(
        "Enter: save | Esc: cancel",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(help, chunks[4]);
}
