use yeet_buffer::{
    message::{BufferMessage, CursorDirection, Search, TextModification},
    model::{ansi::Ansi, BufferLine, CommandMode, Mode, SearchDirection},
    update::update_buffer,
};
use yeet_keymap::message::{KeymapMessage, PrintContent};

use crate::{
    action::{self, Action},
    event::Message,
    model::Model,
    update::{
        register::get_register,
        search::{clear_search, search_in_buffers},
    },
};

use super::set_viewport_dimensions;

pub fn update_commandline(model: &mut Model, message: Option<&BufferMessage>) -> Vec<Action> {
    let command_mode = match &model.mode {
        Mode::Command(it) => it,
        Mode::Insert | Mode::Navigation | Mode::Normal => return Vec::new(),
    };

    let commandline = &mut model.commandline;
    let buffer = &mut commandline.buffer;
    let cursor = &mut commandline.cursor;
    let viewport = &mut commandline.viewport;

    set_viewport_dimensions(viewport, &commandline.layout.buffer);

    if let Some(message) = message {
        match command_mode {
            CommandMode::Command | CommandMode::Search(_) => {
                update_buffer(viewport, cursor, &model.mode, buffer, message);
            }
            CommandMode::PrintMultiline => {}
        }
    }

    Vec::new()
}

pub fn update_commandline_on_modification(
    model: &mut Model,
    repeat: &usize,
    modification: &TextModification,
) -> Vec<Action> {
    let command_mode = match &model.mode {
        Mode::Command(it) => it,
        Mode::Insert | Mode::Navigation | Mode::Normal => return Vec::new(),
    };

    let commandline = &mut model.commandline;
    let buffer = &mut commandline.buffer;
    let cursor = &mut commandline.cursor;
    let viewport = &mut commandline.viewport;

    set_viewport_dimensions(viewport, &commandline.layout.buffer);

    match command_mode {
        CommandMode::Command | CommandMode::Search(_) => {
            let mut actions = Vec::new();
            if let &TextModification::DeleteMotion(_, CursorDirection::Left) = modification {
                if let Some(line) = buffer.lines.last() {
                    if line.content.is_empty() {
                        actions.push(action::emit_keymap(KeymapMessage::Buffer(
                            BufferMessage::ChangeMode(
                                model.mode.clone(),
                                get_mode_after_command(&model.mode_before),
                            ),
                        )));
                    }
                }
            };

            update_buffer(
                viewport,
                cursor,
                &model.mode,
                buffer,
                &BufferMessage::Modification(*repeat, modification.clone()),
            );

            if matches!(model.mode, Mode::Command(CommandMode::Search(_))) {
                let term = model
                    .commandline
                    .buffer
                    .lines
                    .last()
                    .map(|bl| bl.content.to_stripped_string());

                search_in_buffers(model, term);
            }

            actions
        }
        CommandMode::PrintMultiline => {
            let mut messages = Vec::new();
            if let TextModification::Insert(cnt) = modification {
                let action = if matches!(cnt.as_str(), ":" | "/" | "?") {
                    model.mode = Mode::Command(match cnt.as_str() {
                        ":" => CommandMode::Command,
                        "/" => CommandMode::Search(SearchDirection::Down),
                        "?" => CommandMode::Search(SearchDirection::Up),
                        _ => unreachable!(),
                    });

                    let bufferline = BufferLine {
                        prefix: Some(cnt.to_string()),
                        ..Default::default()
                    };

                    buffer.lines.pop();
                    buffer.lines.push(bufferline);

                    Message::Rerender
                } else {
                    update_buffer(
                        viewport,
                        cursor,
                        &model.mode,
                        buffer,
                        &BufferMessage::SetContent(vec![]),
                    );

                    Message::Keymap(KeymapMessage::Buffer(BufferMessage::ChangeMode(
                        model.mode.clone(),
                        get_mode_after_command(&model.mode_before),
                    )))
                };

                messages.push(Action::EmitMessages(vec![action]));
            }

            messages
        }
    }
}

pub fn update_commandline_on_execute(model: &mut Model) -> Vec<Action> {
    let command_mode = match &model.mode {
        Mode::Command(it) => it,
        Mode::Insert | Mode::Navigation | Mode::Normal => return Vec::new(),
    };

    let messages = match command_mode {
        CommandMode::Command => {
            if let Some(cmd) = model.commandline.buffer.lines.last() {
                // TODO: add command history and show previous command not current (this enables g: as well)
                model.register.command = Some(cmd.content.to_stripped_string());

                vec![Message::Keymap(KeymapMessage::ExecuteCommandString(
                    cmd.content.to_stripped_string(),
                ))]
            } else {
                Vec::new()
            }
        }
        CommandMode::PrintMultiline => {
            vec![Message::Keymap(KeymapMessage::Buffer(
                BufferMessage::ChangeMode(
                    model.mode.clone(),
                    get_mode_after_command(&model.mode_before),
                ),
            ))]
        }
        CommandMode::Search(direction) => {
            model.register.searched = model
                .commandline
                .buffer
                .lines
                .last()
                .map(|bl| (direction.clone(), bl.content.to_stripped_string()));

            if model.register.searched.is_none() {
                clear_search(model);
            }

            vec![
                Message::Keymap(KeymapMessage::Buffer(BufferMessage::ChangeMode(
                    model.mode.clone(),
                    get_mode_after_command(&model.mode_before),
                ))),
                Message::Keymap(KeymapMessage::Buffer(BufferMessage::MoveCursor(
                    1,
                    CursorDirection::Search(Search::Next),
                ))),
            ]
        }
    };

    update_buffer(
        &mut model.commandline.viewport,
        &mut model.commandline.cursor,
        &model.mode,
        &mut model.commandline.buffer,
        &BufferMessage::SetContent(vec![]),
    );

    vec![Action::EmitMessages(messages)]
}

pub fn leave_commandline(model: &mut Model) -> Vec<Action> {
    if matches!(model.mode, Mode::Command(CommandMode::Search(_))) {
        let content = get_register(&model.register, &'/');
        search_in_buffers(model, content);
    }

    update_buffer(
        &mut model.commandline.viewport,
        &mut model.commandline.cursor,
        &model.mode,
        &mut model.commandline.buffer,
        &BufferMessage::SetContent(vec![]),
    );

    vec![action::emit_keymap(KeymapMessage::Buffer(
        BufferMessage::ChangeMode(
            model.mode.clone(),
            get_mode_after_command(&model.mode_before),
        ),
    ))]
}

// TODO: buffer messages till command mode left
pub fn print_in_commandline(model: &mut Model, content: &[PrintContent]) -> Vec<Action> {
    let commandline = &mut model.commandline;
    let viewport = &mut commandline.viewport;

    set_viewport_dimensions(viewport, &commandline.layout.buffer);

    commandline.buffer.lines = content
        .iter()
        .map(|content| match content {
            PrintContent::Default(cntnt) => BufferLine {
                content: Ansi::new(&cntnt.to_string()),
                ..Default::default()
            },
            PrintContent::Error(cntnt) => BufferLine {
                content: Ansi::new(&format!("\x1b[31m{}\x1b[39m", cntnt)),
                ..Default::default()
            },
            PrintContent::Information(cntnt) => BufferLine {
                content: Ansi::new(&format!("\x1b[92m{}\x1b[39m", cntnt)),
                ..Default::default()
            },
        })
        .collect();

    let actions = if commandline.buffer.lines.len() > 1 {
        let content = "Press ENTER or type command to continue";
        commandline.buffer.lines.push(BufferLine {
            content: Ansi::new(&format!("\x1b[94m{}\x1b[39m", content)),
            ..Default::default()
        });

        if model.mode.is_command() {
            model.mode = Mode::Command(CommandMode::PrintMultiline);
        }

        vec![action::emit_keymap(KeymapMessage::Buffer(
            BufferMessage::ChangeMode(
                model.mode.clone(),
                Mode::Command(CommandMode::PrintMultiline),
            ),
        ))]
    } else {
        Vec::new()
    };

    update_buffer(
        &mut commandline.viewport,
        &mut commandline.cursor,
        &model.mode,
        &mut commandline.buffer,
        &BufferMessage::MoveCursor(1, CursorDirection::Bottom),
    );
    update_buffer(
        &mut commandline.viewport,
        &mut commandline.cursor,
        &model.mode,
        &mut commandline.buffer,
        &BufferMessage::MoveCursor(1, CursorDirection::LineEnd),
    );

    actions
}

fn get_mode_after_command(mode_before: &Option<Mode>) -> Mode {
    if let Some(mode) = mode_before {
        match mode {
            Mode::Command(_) => unreachable!(),
            Mode::Insert | Mode::Normal => Mode::Normal,
            Mode::Navigation => Mode::Navigation,
        }
    } else {
        Mode::default()
    }
}
