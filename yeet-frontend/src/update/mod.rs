use ratatui::prelude::Rect;
use yeet_keymap::message::{Message, Mode};

use crate::{
    action::Action,
    model::{
        buffer::{viewport::ViewPort, Buffer},
        Model,
    },
    settings::Settings,
};

mod buffer;
mod command;
pub mod commandline;
mod current;
mod enumeration;
mod history;
mod modification;
mod navigation;
mod parent;
mod path;
mod preview;
mod register;

pub fn update(settings: &Settings, model: &mut Model, message: &Message) -> Option<Vec<Action>> {
    match message {
        Message::Buffer(msg) => modification::buffer(model, msg),
        Message::EnumerationChanged(path, contents) => enumeration::changed(model, path, contents),
        Message::EnumerationFinished(path) => enumeration::finished(model, path),
        Message::ExecuteCommand => commandline::update_on_execute(model),
        Message::ExecuteCommandString(command) => Some(command::execute(command, model)),
        Message::KeySequenceChanged(sequence) => {
            model.key_sequence = sequence.clone();
            commandline::update(model, None);
            None
        }
        Message::NavigateToParent => navigation::parent(model),
        Message::NavigateToPath(path) => navigation::path(model, path),
        Message::NavigateToSelected => navigation::selected(model),
        Message::OpenSelected => current::open(model, settings),
        Message::PasteRegister(register) => register::paste(model, register),
        Message::PathRemoved(path) => path::remove(model, path),
        Message::PathsAdded(paths) => path::add(model, paths),
        Message::PathsWriteFinished(paths) => register::add(model, paths),
        Message::PreviewLoaded(path, content) => preview::update(model, path, content),
        Message::Print(content) => commandline::print(model, content),
        Message::Rerender => None,
        Message::Resize(x, y) => Some(vec![Action::Resize(*x, *y)]),
        Message::Quit => Some(vec![Action::Quit(None)]),
        Message::YankSelected => register::yank(model),
    }
}

fn set_viewport_dimensions(vp: &mut ViewPort, rect: &Rect) {
    vp.height = usize::from(rect.height);
    vp.width = usize::from(rect.width);
}

fn sort_content(mode: &Mode, model: &mut Buffer) {
    model.lines.sort_unstable_by(|a, b| {
        a.content
            .to_ascii_uppercase()
            .cmp(&b.content.to_ascii_uppercase())
    });
    buffer::cursor::validate(mode, model);
}
