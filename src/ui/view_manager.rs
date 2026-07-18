use crate::ui::{App, ViewEditField};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use tui_textarea::TextArea;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let code = key.code;
    if app.view_edit.is_some() {
        match code {
            KeyCode::Esc => {
                app.view_edit = None;
            }
            KeyCode::Enter => {
                if let (Some(idx), Some(textarea)) =
                    (app.view_manager_state.selected(), &app.view_edit)
                {
                    if let Some(view) = app.views.get_mut(idx) {
                        let text = textarea.lines()[0].clone();
                        match app.editing_view_field {
                            ViewEditField::Name => view.name = text,
                            ViewEditField::Query => view.query = text,
                            ViewEditField::SortBy => view.sort_by = text,
                            ViewEditField::GroupBy => view.group_by = text,
                        }
                    }
                }
                app.view_edit = None;
                app.save_config();
            }
            _ => {
                if let Some(textarea) = &mut app.view_edit {
                    textarea.input(key);
                }
            }
        }
        return;
    }

    match code {
        KeyCode::Esc => {
            app.show_view_manager = false;
        }
        KeyCode::Char('j') | KeyCode::Down if !app.views.is_empty() => {
            let i = app.view_manager_state.selected().unwrap_or(0);
            let next = if i >= app.views.len() - 1 { 0 } else { i + 1 };
            app.view_manager_state.select(Some(next));
        }
        KeyCode::Char('k') | KeyCode::Up if !app.views.is_empty() => {
            let i = app.view_manager_state.selected().unwrap_or(0);
            let prev = if i == 0 { app.views.len() - 1 } else { i - 1 };
            app.view_manager_state.select(Some(prev));
        }
        KeyCode::Char('e') => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    let mut textarea = TextArea::new(vec![view.name.clone()]);
                    textarea.set_cursor_line_style(ratatui::style::Style::default());
                    app.view_edit = Some(textarea);
                    app.editing_view_field = ViewEditField::Name;
                }
            }
        }
        KeyCode::Char('d') if app.views.len() > 1 => {
            if let Some(idx) = app.view_manager_state.selected() {
                if idx < app.views.len() {
                    app.views.remove(idx);
                    let new_sel = if idx >= app.views.len() {
                        app.views.len() - 1
                    } else {
                        idx
                    };
                    app.view_manager_state.select(Some(new_sel));
                    app.save_config();
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    app.config.defaults.view = view.name.clone();
                    app.save_config();
                }
            }
        }
        KeyCode::Enter => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    app.current_view = view.clone();
                    app.show_view_manager = false;
                    app.filter_text.clear();
                    app.dirty = true;
                }
            }
        }
        _ => {}
    }
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    let popup_width = 60.min(area.width - 4);
    let popup_height = (app.views.len() as u16 + 5).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    if let Some(textarea) = &app.view_edit {
        let field_name = match app.editing_view_field {
            ViewEditField::Name => "Name",
            ViewEditField::Query => "Query",
            ViewEditField::SortBy => "Sort By",
            ViewEditField::GroupBy => "Group By",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Edit View")
            .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(block, popup_area);

        let inner = popup_area.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(inner);

        let view_name = app
            .views
            .get(app.view_manager_state.selected().unwrap_or(0))
            .map(|v| v.name.as_str())
            .unwrap_or("");
        let title = Paragraph::new(format!("Editing view: {}", view_name));
        frame.render_widget(title, chunks[0]);

        let label = Paragraph::new(vec![Line::from(vec![Span::styled(
            format!("{}: ", field_name),
            Style::default().fg(Color::Gray),
        )])]);
        frame.render_widget(label, chunks[1]);

        frame.render_widget(textarea, chunks[2]);

        let help = Paragraph::new(Span::styled(
            "Enter: save | Esc: cancel",
            Style::default().fg(Color::Gray),
        ));
        frame.render_widget(help, chunks[3]);
    } else {
        let items: Vec<ListItem> = app
            .views
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let style = if Some(i) == app.view_manager_state.selected() {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if v.name == app.current_view.name {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                let prefix = if v.name == app.current_view.name {
                    "* "
                } else {
                    "  "
                };
                let suffix = if v.name == app.config.defaults.view {
                    " (default)"
                } else {
                    ""
                };
                ListItem::new(Line::from(format!("{}{}{}", prefix, v.name, suffix))).style(style)
            })
            .collect();

        let help_line = Line::from(Span::styled(
            "Enter: switch | a: add | e: edit | d: del | s: set default | Esc: close",
            Style::default().fg(Color::Gray),
        ));

        let help_item = ListItem::new(help_line);
        let mut all_items = items;
        all_items.push(help_item);

        let list = List::new(all_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Manage Views")
                .style(Style::default().bg(Color::DarkGray)),
        );

        let mut state = ListState::default();
        state.select(app.view_manager_state.selected());
        frame.render_stateful_widget(list, popup_area, &mut state);
    }
}
