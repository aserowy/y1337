use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use yeet_buffer::model::undo::{self, BufferChanged};

use crate::model::Model;

pub fn view(model: &Model, frame: &mut Frame, rect: Rect) {
    let changes = get_changes_content(model);
    let position = get_position_content(model);

    let content = model.files.current.path.to_str().unwrap_or("");
    let style = Style::default().fg(Color::Gray);
    let span = Span::styled(content, style);
    let path = Line::from(span);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(path.width() as u16),
            Constraint::Length(3),
            Constraint::Min(changes.width() as u16),
            Constraint::Length(position.width() as u16),
        ])
        .split(rect);

    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        rect,
    );

    frame.render_widget(Paragraph::new(path), layout[0]);
    frame.render_widget(Paragraph::new(changes), layout[2]);
    frame.render_widget(Paragraph::new(position), layout[3]);
}

fn get_position_content(model: &Model) -> Line {
    let count = model.files.current.buffer.lines.len();
    let current_position = model
        .files
        .current_cursor
        .as_ref()
        .map(|crsr| crsr.vertical_index + 1);

    let mut content = Vec::new();
    if let Some(mut position) = current_position {
        if count == 0 {
            position = 0;
        }

        content.push(Span::styled(
            format!("{}/", position),
            Style::default().fg(Color::Gray),
        ));
    }

    content.push(Span::styled(
        format!("{}", count),
        Style::default().fg(Color::Gray),
    ));

    Line::from(content)
}

fn get_changes_content(model: &Model) -> Line {
    let modifications = model.files.current.buffer.undo.get_uncommited_changes();
    let changes = undo::consolidate_modifications(&modifications);

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
