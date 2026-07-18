use crate::ui::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use tui_textarea::{CursorMove, TextArea};

pub const FIELD_NAME: usize = 0;
pub const FIELD_QUERY: usize = 1;
pub const FIELD_SORT_BY: usize = 2;
pub const FIELD_GROUP_BY: usize = 3;
const FIELD_COUNT: usize = 4;

pub struct ViewForm {
    pub fields: [TextArea<'static>; FIELD_COUNT],
    pub focus: usize,
    pub editing_index: Option<usize>, // None = adding a new view
}

fn new_field(text: &str) -> TextArea<'static> {
    let lines = if text.is_empty() {
        vec![String::new()]
    } else {
        text.lines().map(|l| l.to_string()).collect()
    };
    let mut textarea = TextArea::new(lines);
    textarea.set_cursor_line_style(Style::default());
    textarea.move_cursor(CursorMove::End);
    textarea
}

impl ViewForm {
    fn empty() -> Self {
        let mut form = Self {
            fields: [new_field(""), new_field(""), new_field(""), new_field("")],
            focus: 0,
            editing_index: None,
        };
        form.update_focus_styles();
        form
    }

    fn for_view(idx: usize, view: &crate::view::View) -> Self {
        let mut form = Self {
            fields: [
                new_field(&view.name),
                new_field(&view.query),
                new_field(&view.sort_by),
                new_field(&view.group_by),
            ],
            focus: 0,
            editing_index: Some(idx),
        };
        form.update_focus_styles();
        form
    }

    fn update_focus_styles(&mut self) {
        for (i, field) in self.fields.iter_mut().enumerate() {
            if i == self.focus {
                field.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            } else {
                field.set_cursor_style(Style::default());
            }
        }
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.view_form.is_some() {
        handle_form_key(app, key);
        return;
    }

    let code = key.code;
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
        KeyCode::Char('a') => {
            app.view_form = Some(ViewForm::empty());
        }
        KeyCode::Char('e') => {
            if let Some(idx) = app.view_manager_state.selected() {
                if let Some(view) = app.views.get(idx) {
                    app.view_form = Some(ViewForm::for_view(idx, view));
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

fn handle_form_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.view_form = None;
        }
        KeyCode::Tab => {
            if let Some(form) = &mut app.view_form {
                form.focus = (form.focus + 1) % FIELD_COUNT;
                form.update_focus_styles();
            }
        }
        KeyCode::BackTab => {
            if let Some(form) = &mut app.view_form {
                form.focus = (form.focus + FIELD_COUNT - 1) % FIELD_COUNT;
                form.update_focus_styles();
            }
        }
        KeyCode::Enter if !key.modifiers.contains(KeyModifiers::ALT) => {
            save_form(app);
        }
        _ => {
            // Includes Alt+Enter, which tui-textarea turns into a newline
            // (multi-line queries).
            if let Some(form) = &mut app.view_form {
                form.fields[form.focus].input(key);
            }
        }
    }
}

fn save_form(app: &mut App) {
    let Some(form) = &app.view_form else {
        return;
    };
    let name = form.fields[FIELD_NAME].lines()[0].trim().to_string();
    let editing_index = form.editing_index;
    let query = form.fields[FIELD_QUERY].lines().join("\n");
    let sort_by = form.fields[FIELD_SORT_BY].lines()[0].clone();
    let group_by = form.fields[FIELD_GROUP_BY].lines()[0].clone();

    if name.is_empty() {
        app.status_message = Some("View name required".to_string());
        return;
    }

    let duplicate = app
        .views
        .iter()
        .enumerate()
        .any(|(i, v)| v.name == name && Some(i) != editing_index);
    if duplicate {
        app.status_message = Some(format!("View '{}' already exists", name));
        return;
    }

    match editing_index {
        Some(idx) => {
            let was_current = app.views[idx].name == app.current_view.name;
            {
                let view = &mut app.views[idx];
                view.name = name;
                view.query = query;
                view.sort_by = sort_by;
                view.group_by = group_by;
            }
            if was_current {
                app.current_view = app.views[idx].clone();
                app.dirty = true;
            }
        }
        None => {
            app.views
                .push(crate::view::View::new(&name, &query, &sort_by, &group_by));
        }
    }

    app.save_config();
    app.view_form = None;
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    if app.view_form.is_some() {
        draw_form(frame, app);
    } else if app.show_view_manager {
        draw_list(frame, app);
    }
}

fn draw_list(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    let popup_width = 60.min(area.width - 4);
    let popup_height = (app.views.len() as u16 + 5).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

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

fn draw_form(frame: &mut ratatui::Frame, app: &App) {
    let Some(form) = &app.view_form else {
        return;
    };
    let area = frame.area();
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 13.min(area.height.saturating_sub(4));
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let title = if form.editing_index.is_some() {
        "Edit View"
    } else {
        "Add View"
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, popup_area);

    let inner = popup_area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // name row
            Constraint::Length(1), // query label
            Constraint::Length(3), // query textarea
            Constraint::Length(1), // sort_by row
            Constraint::Length(1), // group_by row
            Constraint::Length(1), // blank
            Constraint::Length(1), // help
        ])
        .split(inner);

    draw_inline_field(
        frame,
        &form.fields[FIELD_NAME],
        "Name",
        form.focus == FIELD_NAME,
        chunks[0],
    );

    frame.render_widget(field_label("Query", form.focus == FIELD_QUERY), chunks[1]);
    frame.render_widget(&form.fields[FIELD_QUERY], chunks[2]);

    draw_inline_field(
        frame,
        &form.fields[FIELD_SORT_BY],
        "Sort by",
        form.focus == FIELD_SORT_BY,
        chunks[3],
    );
    draw_inline_field(
        frame,
        &form.fields[FIELD_GROUP_BY],
        "Group by",
        form.focus == FIELD_GROUP_BY,
        chunks[4],
    );

    let help = Paragraph::new(Span::styled(
        "Tab: next field | Enter: save | Esc: cancel",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(help, chunks[6]);
}

fn field_label(label: &str, focused: bool) -> Paragraph<'static> {
    let style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    Paragraph::new(format!("{}:", label)).style(style)
}

fn draw_inline_field(
    frame: &mut ratatui::Frame,
    textarea: &TextArea,
    label: &str,
    focused: bool,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(10), Constraint::Min(1)])
        .split(area);
    frame.render_widget(field_label(label, focused), chunks[0]);
    frame.render_widget(textarea, chunks[1]);
}
