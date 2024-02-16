use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::model::{buffer::undo::BufferChanged, Model};

pub fn view(model: &mut Model, frame: &mut Frame, rect: Rect) {
    let changes = get_changes_content(model);
    let position = get_position_content(model);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Min(5),
            Constraint::Min(1),
            Constraint::Max(position.width() as u16),
        ])
        .split(rect);

    let content = model.current.path.to_str().unwrap_or("");
    let style = Style::default().fg(Color::Green);
    let span = Span::styled(content, style);

    frame.render_widget(Paragraph::new(Line::from(span)), layout[0]);
    frame.render_widget(Paragraph::new(changes), layout[2]);
    frame.render_widget(Paragraph::new(position), layout[3]);
}

fn get_position_content(model: &Model) -> Line {
    let count = model.current.buffer.lines.len();
    let current_position = model
        .current
        .buffer
        .cursor
        .as_ref()
        .and_then(|crsr| Some(crsr.vertical_index + 1));

    let mut content = Vec::new();
    if let Some(position) = current_position {
        content.push(Span::styled(
            format!("{}/", position),
            Style::default().fg(Color::Green),
        ));
    }

    content.push(Span::styled(
        format!("{}", count),
        Style::default().fg(Color::Green),
    ));

    Line::from(content)
}

fn get_changes_content(model: &Model) -> Line {
    let modifications = model.current.buffer.undo.get_uncommited_changes();
    let changes = crate::model::buffer::undo::consolidate(&modifications);

    let (mut added, mut changed, mut removed) = (0, 0, 0);
    for change in changes {
        match change {
            BufferChanged::Content(_, _, _) => changed += 1,
            BufferChanged::LineAdded(_, _) => added += 1,
            BufferChanged::LineRemoved(_, _) => removed += 1,
        }
    }

    let mut content = Vec::new();
    if added > 0 {
        content.push(Span::styled(
            format!("+{} ", added),
            Style::default().fg(Color::Green),
        ));
    }

    if changed > 0 {
        content.push(Span::styled(
            format!("~{} ", changed),
            Style::default().fg(Color::Yellow),
        ));
    }

    if removed > 0 {
        content.push(Span::styled(
            format!("-{} ", removed),
            Style::default().fg(Color::Red),
        ));
    }

    Line::from(content)
}
