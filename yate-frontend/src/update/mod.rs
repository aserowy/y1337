use ratatui::prelude::Rect;
use yate_keymap::message::{Message, Mode};

use crate::{
    event::PostRenderAction,
    layout::AppLayout,
    model::{buffer::ViewPort, Model},
    task::Task,
};

mod buffer;
mod commandline;
mod current;
mod history;
mod parent;
mod path;
mod preview;

pub fn update(
    model: &mut Model,
    layout: &AppLayout,
    message: &Message,
) -> Option<Vec<PostRenderAction>> {
    match message {
        Message::ChangeKeySequence(sequence) => {
            model.key_sequence = sequence.clone();

            None
        }
        Message::ChangeMode(from, to) => {
            if from == to {
                return None;
            }

            model.mode = to.clone();

            match from {
                Mode::Command => {
                    buffer::unfocus_buffer(&mut model.commandline);
                    commandline::update(model, layout, message);
                }
                Mode::Insert => {
                    buffer::unfocus_buffer(&mut model.current_directory);
                }
                Mode::Normal => {
                    buffer::unfocus_buffer(&mut model.current_directory);
                }
            }

            match to {
                Mode::Command => {
                    buffer::focus_buffer(&mut model.commandline);
                    commandline::update(model, layout, message);
                }
                Mode::Insert => {
                    buffer::focus_buffer(&mut model.current_directory);
                }
                Mode::Normal => {
                    // NOTE: add file modification handling
                    buffer::focus_buffer(&mut model.current_directory);
                }
            }

            Some(vec![PostRenderAction::ModeChanged(to.clone())])
        }
        Message::ExecuteCommand => {
            if let Some(cmd) = model.commandline.lines.first() {
                let post_render_actions = match cmd.content.as_str() {
                    "histopt" => Some(vec![PostRenderAction::Task(Task::OptimizeHistory)]),
                    "q" => update(model, layout, &Message::Quit),
                    // TODO: add notification in cmd line?
                    _ => None,
                };

                let mode_changed_actions = update(
                    model,
                    layout,
                    &Message::ChangeMode(model.mode.clone(), Mode::Normal),
                );

                Some(
                    post_render_actions
                        .into_iter()
                        .flatten()
                        .chain(mode_changed_actions.into_iter().flatten())
                        .collect(),
                )
            } else {
                None
            }
        }
        Message::Modification(_) => match model.mode {
            Mode::Command => {
                commandline::update(model, layout, message);

                None
            }
            Mode::Insert => current::update(model, layout, message),
            Mode::Normal => current::update(model, layout, message),
        },
        Message::MoveCursor(_, _) => match model.mode {
            Mode::Command => {
                commandline::update(model, layout, message);

                None
            }
            Mode::Insert => {
                let actions = current::update(model, layout, message);
                preview::update(model, layout, message);

                actions
            }
            Mode::Normal => {
                let actions = current::update(model, layout, message);
                preview::update(model, layout, message);

                actions
            }
        },
        Message::MoveViewPort(_) => match model.mode {
            Mode::Command => {
                commandline::update(model, layout, message);

                None
            }
            Mode::Insert => {
                let actions = current::update(model, layout, message);
                preview::update(model, layout, message);

                actions
            }
            Mode::Normal => {
                let actions = current::update(model, layout, message);
                preview::update(model, layout, message);

                actions
            }
        },
        Message::Refresh => {
            let actions = current::update(model, layout, message);
            commandline::update(model, layout, message);
            parent::update(model, layout, message);
            preview::update(model, layout, message);

            actions
        }
        Message::SelectCurrent => {
            if let Some(target) = path::get_selected_path(model) {
                if !target.is_dir() {
                    return None;
                }

                model.current_path = target.clone();

                let actions = current::update(model, layout, message);

                history::set_cursor_index(
                    &model.current_path,
                    &model.history,
                    &mut model.current_directory,
                );

                parent::update(model, layout, message);
                preview::update(model, layout, message);

                model.history.add(&target);

                actions
            } else {
                None
            }
        }
        Message::SelectParent => {
            if let Some(parent) = &model.current_path.parent() {
                model.current_path = parent.to_path_buf();

                let actions = current::update(model, layout, message);

                history::set_cursor_index(
                    &model.current_path,
                    &model.history,
                    &mut model.current_directory,
                );

                parent::update(model, layout, message);
                preview::update(model, layout, message);

                actions
            } else {
                None
            }
        }
        Message::Quit => Some(vec![
            PostRenderAction::OptimizeHistory,
            PostRenderAction::Quit,
        ]),
    }
}

fn set_viewport_dimensions(vp: &mut ViewPort, rect: &Rect) {
    vp.height = usize::from(rect.height);
    vp.width = usize::from(rect.width);
}
