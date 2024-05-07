use crate::{action::Action, model::Model};

use yeet_buffer::{
    message::{BufferMessage, CursorDirection, Search},
    model::{CommandMode, Mode, SearchDirection},
    update::update_buffer,
};

use super::{
    commandline::{update_commandline, update_commandline_on_modification},
    history::get_selection_from_history,
    mode::change_mode,
    preview::{set_preview_to_selected, validate_preview_viewport},
    register::{get_direction_from_search_register, get_register},
    save::persist_path_changes,
    search::search_in_buffers,
    set_viewport_dimensions,
};

// TODO: refactor like update mod into function per Message match
#[tracing::instrument(skip(model, msg))]
pub fn update_with_buffer_message(model: &mut Model, msg: &BufferMessage) -> Vec<Action> {
    match msg {
        BufferMessage::ChangeMode(from, to) => change_mode(model, from, to),
        BufferMessage::Modification(repeat, modification) => match model.mode {
            Mode::Command(CommandMode::Command) | Mode::Command(CommandMode::PrintMultiline) => {
                update_commandline_on_modification(model, repeat, modification)
            }
            Mode::Command(_) => {
                let actions = update_commandline_on_modification(model, repeat, modification);

                let term = model
                    .commandline
                    .buffer
                    .lines
                    .last()
                    .map(|bl| bl.content.clone());

                search_in_buffers(model, term);

                actions
            }
            Mode::Insert | Mode::Normal => {
                update_current(model, msg);

                let mut actions = Vec::new();
                if let Some(path) = set_preview_to_selected(model) {
                    validate_preview_viewport(model);

                    let selection =
                        get_selection_from_history(&model.history, &path).map(|s| s.to_owned());
                    actions.push(Action::Load(path, selection));
                }

                actions
            }
            Mode::Navigation => Vec::new(),
        },
        BufferMessage::MoveCursor(rpt, mtn) => match &model.mode {
            Mode::Command(_) => update_commandline(model, Some(msg)),
            Mode::Insert | Mode::Navigation | Mode::Normal => {
                if let CursorDirection::Search(dr) = mtn {
                    let term = get_register(&model.register, &'/');
                    search_in_buffers(model, term);

                    let current_dr = match get_direction_from_search_register(&model.register) {
                        Some(it) => it,
                        None => return Vec::new(),
                    };

                    let dr = match (dr, current_dr) {
                        (Search::Next, SearchDirection::Down) => Search::Next,
                        (Search::Next, SearchDirection::Up) => Search::Previous,
                        (Search::Previous, SearchDirection::Down) => Search::Previous,
                        (Search::Previous, SearchDirection::Up) => Search::Next,
                    };

                    let msg = BufferMessage::MoveCursor(*rpt, CursorDirection::Search(dr.clone()));
                    update_current(model, &msg);
                } else {
                    update_current(model, msg);
                };

                let mut actions = Vec::new();
                if let Some(path) = set_preview_to_selected(model) {
                    validate_preview_viewport(model);

                    let selection =
                        get_selection_from_history(&model.history, &path).map(|s| s.to_owned());
                    actions.push(Action::Load(path, selection));
                }

                actions
            }
        },
        BufferMessage::MoveViewPort(_) => match model.mode {
            Mode::Command(_) => update_commandline(model, Some(msg)),
            Mode::Insert | Mode::Navigation | Mode::Normal => {
                update_current(model, msg);

                let mut actions = Vec::new();
                if let Some(path) = set_preview_to_selected(model) {
                    validate_preview_viewport(model);

                    let selection =
                        get_selection_from_history(&model.history, &path).map(|s| s.to_owned());
                    actions.push(Action::Load(path, selection));
                }

                actions
            }
        },
        BufferMessage::SaveBuffer => persist_path_changes(model),

        BufferMessage::RemoveLine(_)
        | BufferMessage::ResetCursor
        | BufferMessage::SetContent(_)
        | BufferMessage::SetCursorToLineContent(_)
        | BufferMessage::SortContent(_) => unreachable!(),
    }
}

// TODO: move out of this mod to prevent cycling dep
pub fn update_current(model: &mut Model, message: &BufferMessage) {
    let buffer = &mut model.files.current.buffer;
    let layout = &model.layout.current;

    set_viewport_dimensions(&mut buffer.view_port, layout);
    update_buffer(&model.mode, buffer, message);
}
