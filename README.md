# yeet or y337

## the name, the vision

Yet Another Astoundingly Hackable, Keyboard-Controlled, Efficient, Versatile,
Interactive, Fast, Elmish, Minimalistic, and Superlative File Explorer with
Vim-Inspired Keybindings, Infused with the Magic of Lua, Allowing Users to Extend
Its Functionality, Shape Its Behavior, and Create Customized Workflows Tailored
to Their Unique Needs!

In short: y337

## shortcuts

### changing modes

In every mode `esc` switches to the next 'level' mode. The order is:

navigation < normal < insert

Exceptions to this order is the command mode. Leaving this mode will restore the
previous one.

When transition from normal to navigation all changes to the filesystem will get
persisted. Thus, changes in insert and normal are handled like unsaved buffer changes
and are not present on the file system till `:w` gets called or the mode changes
to navigation.

### navigation mode

| keys       | action                                                    |
| ---------- | --------------------------------------------------------- |
| h, l       | navigating the file tree                                  |
| j, k       | navigating the current directory                          |
| gh         | goto home directory                                       |
| m          | go into normal mode                                       |
| dd         | go into normal and delete the current line                |
| o, O       | add a new line and change to insert mode                  |
| i, a       | change to insert mode                                     |
| I, A       | jump to line start/end and change to insert mode          |
| :          | change to command mode                                    |
| zt, zz, zb | move viewport to start, center, bottom of cursor position |
| C-u, C-d   | move viewport half screen up/down                         |

### normal mode

| keys       | action                                                    |
| ---------- | --------------------------------------------------------- |
| h, l       | move cursor left/right                                    |
| 0, $       | move cursor to line start/end                             |
| j, k       | navigating the current directory                          |
| m          | go into normal mode                                       |
| dd         | delete the current line                                   |
| o, O       | add a new line and change to insert mode                  |
| i, a       | change to insert mode                                     |
| I, A       | jump to line start/end and change to insert mode          |
| :          | change to command mode                                    |
| zt, zz, zb | move viewport to start, center, bottom of cursor position |
| C-u, C-d   | move viewport half screen up/down                         |

## architecture overview

### yeet

The main crate is handling frontend and backend and resolves cli arguments to
pass them to the relevant components.

### yeet-frontend

The frontend follows an elm architecture with one exception: The model is
mutable and will not get created every update.

frontend.rs holds the lifecycle of the tui. It starts an event stream to
enable non lockable operations. This stream is implemented in event.rs and
translates multiple event emitter like terminal interaction with crossterm into
AppEvents.

layout.rs defines the overall app layout, which is used by all view functions.

The modules model, update and view represent the elm philosophy. Messages
are defined in yeet-keymap to prevent cycling dependencies.

### yeet-keymap

This crate holds all key relevant features. The MessageResolver uses buffer
and tree to resolve possible messages, which follow the elm architecture to
modify the model.

tree uses the keymap to build a key tree structure. Thus, in keymap all
key combinations are mapped indirectly to messages.

conversion translates crossterm key events to the yeet-keymap
representation.

## faq

### opening files in linux does nothing

yeet utilizes `xdg-open` to start files. Thus, not opening anything probably lies
in a misconfigured mime setup. Check `~/.local/share/applications/` for invalid entries.
Some programs causing problems regularly. Im looking at you `wine`...
