use std::collections::HashMap;
use std::fmt::Display;
use std::io::{stdout, Write};
use std::time::Duration;

use crossterm::cursor::MoveTo;
use crossterm::event::{
    DisableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::style::Color::{DarkBlue, DarkGreen, Reset, Yellow};
use crossterm::style::{
    Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{cursor, event, execute, queue, terminal, ExecutableCommand};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use super::colors::CommandDefinitionColor;
use super::types::{
    CommandChoice, CommandForDisplay, CommandIndex, CycleDirection, UiState, ViewportState,
};
use super::LAST_COMMAND_OPTION;
use crate::command_selection::types::CommandIndex::Normal;
use crate::command_selection::types::CycleDirection::{Down, Up};
use rust_cuts_core::command_definitions::{CommandDefinition, CommandExecutionTemplate};
use rust_cuts_core::error::Result;

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // Disable raw mode on drop
        let _ = disable_raw_mode();
        let mut stdout = stdout();
        let _ = stdout.execute(DisableMouseCapture);
        let _ = stdout.execute(LeaveAlternateScreen);
    }
}

fn redraw_ui(
    ui_state: &UiState,
    indexes_to_display: &[CommandIndex],
    command_lookup: &HashMap<CommandIndex, CommandForDisplay>,
) -> Result<()> {
    let mut stdout = stdout();

    queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

    print_header(ui_state, indexes_to_display.len())?;

    if indexes_to_display.is_empty() {
        queue!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("No matching commands!".to_string()),
            SetAttribute(Attribute::Reset),
            cursor::MoveToNextLine(1)
        )?;
    } else {
        print_commands_with_selection(ui_state, command_lookup, indexes_to_display)?;
    }

    if ui_state.is_filtering {
        queue!(
            stdout,
            SetAttribute(Attribute::Bold),
            Print(format!("Filter: {}", ui_state.filter_text)),
            SetAttribute(Attribute::Reset)
        )?;
    }

    stdout.flush()?;
    Ok(())
}

/// Prompts the user to choose a command from the list
pub fn prompt_for_command_choice(
    command_definitions: &[CommandDefinition],
    last_command: Option<&CommandExecutionTemplate>,
) -> Result<CommandChoice> {
    let mut stdout = stdout();

    stdout.execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let _raw_mode_guard = RawModeGuard; // When this goes out of scope, raw mode and mouse capture is disabled
    stdout.execute(event::EnableMouseCapture)?;

    let mut command_display: HashMap<CommandIndex, CommandForDisplay> = command_definitions
        .iter()
        .enumerate()
        .map(|(i, cd)| (Normal(i), CommandForDisplay::Normal(cd.clone())))
        .collect();

    if let Some(lc) = last_command {
        command_display.insert(CommandIndex::Rerun, CommandForDisplay::Rerun(lc.clone()));
    }
    let (width, height) = terminal::size()?;

    let viewport = ViewportState {
        offset: 0,
        height: height.saturating_sub(2), // Subtract 2 for header and filter line
        width,
    };

    let mut ui_state = UiState {
        selected_index: 0,
        viewport,
        is_filtering: false,
        filter_text: String::new(),
    };

    let matcher = SkimMatcherV2::default();

    let mut indexes_to_display =
        filter_displayed_indexes(&matcher, &command_display, &ui_state.filter_text);

    let mut down_row: Option<u16> = None;

    let mut new_ui_state = Some(ui_state.clone());

    let mut force_initial_draw = true;

    loop {
        // Only check for UI state changes if we have a new UI state
        let should_redraw = force_initial_draw
            || if let Some(current_ui_state) = &new_ui_state {
                *current_ui_state != ui_state
            } else {
                false
            };

        force_initial_draw = false;

        if should_redraw {
            // Get the current state to work with (from new_ui_state, which we know exists now)
            let current_ui_state = new_ui_state.unwrap();

            if ui_state.filter_text != current_ui_state.filter_text {
                indexes_to_display = filter_displayed_indexes(
                    &matcher,
                    &command_display,
                    &current_ui_state.filter_text,
                );
            }

            redraw_ui(&current_ui_state, &indexes_to_display, &command_display)?;

            ui_state = current_ui_state;
            new_ui_state = None;
        }

        if event::poll(Duration::from_millis(500))? {
            match event::read()? {
                Event::Mouse(MouseEvent {
                    kind,
                    row,
                    modifiers,
                    ..
                }) => {
                    if modifiers != KeyModifiers::NONE {
                        continue;
                    }
                    let (new_state, command_choice, clicked_row) = handle_mouse_event(
                        kind,
                        down_row,
                        row,
                        &ui_state,
                        &indexes_to_display,
                        &last_command,
                    );

                    if let Some(command_choice) = command_choice {
                        return Ok(command_choice);
                    }

                    if let Some(state) = new_state {
                        new_ui_state = Some(state);
                    }

                    if let Some(clicked_row) = clicked_row {
                        down_row = Some(clicked_row);
                    }
                }
                Event::Key(key_event) => {
                    let (new_state, command_choice) =
                        handle_key_event(key_event, &ui_state, &indexes_to_display, last_command)?;

                    if let Some(choice) = command_choice {
                        return Ok(choice);
                    }

                    if let Some(state) = new_state {
                        new_ui_state = Some(state);
                    }
                }
                Event::Resize(width, height) => {
                    new_ui_state =
                        Some(handle_resize(width, height, &ui_state, &indexes_to_display));
                }
                Event::FocusGained => {}
                Event::FocusLost => {}
                Event::Paste(_) => {}
            }
        }
    }
}

/// Handle keyboard events in the command selection UI
fn handle_key_event(
    key_event: event::KeyEvent,
    ui_state: &UiState, // Now immutable
    indexes_to_display: &[CommandIndex],
    last_command: Option<&CommandExecutionTemplate>,
) -> Result<(Option<UiState>, Option<CommandChoice>)> {
    match key_event.code {
        KeyCode::Up | KeyCode::Down => {
            let direction = if key_event.code == KeyCode::Up {
                Up
            } else {
                Down
            };

            let new_ui_state = move_selected_index(ui_state, indexes_to_display.len(), &direction);

            Ok((Some(new_ui_state), None))
        }
        KeyCode::Enter => {
            if let Some(command_index) = indexes_to_display.get(ui_state.selected_index) {
                match command_index {
                    Normal(i) => return Ok((None, Some(CommandChoice::Index(*i)))),
                    CommandIndex::Rerun => {
                        if let Some(last_command) = last_command {
                            return Ok((None, Some(CommandChoice::Rerun(last_command.clone()))));
                        };
                    }
                }
            } else {
                execute!(stdout(), Print("\x07"))?;
                stdout().flush()?;
            }
            Ok((None, None))
        }
        KeyCode::Backspace => {
            if !ui_state.filter_text.is_empty() {
                let mut updated_state = ui_state.clone();

                // Remove the last character from the filter text
                updated_state.filter_text.pop();

                return Ok((Some(updated_state), None));
            }
            Ok((None, None))
        }
        KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
            // Handle Ctrl-C to quit
            Ok((None, Some(CommandChoice::Quit)))
        }
        KeyCode::Char(c) if ui_state.is_filtering => {
            let mut new_state = ui_state.clone();
            new_state.filter_text.push(c);
            Ok((Some(new_state), None))
        }
        KeyCode::Esc if ui_state.is_filtering => {
            let mut updated_state = ui_state.clone();
            updated_state.is_filtering = false;
            updated_state.filter_text = "".to_string();
            Ok((Some(updated_state), None))
        }
        KeyCode::Char('/') => {
            let mut updated_state = ui_state.clone();
            updated_state.is_filtering = true;
            Ok((Some(updated_state), None))
        }
        KeyCode::Char('q') => Ok((None, Some(CommandChoice::Quit))),
        KeyCode::Char(LAST_COMMAND_OPTION) => {
            if let Some(last_command) = last_command {
                return Ok((None, Some(CommandChoice::Rerun(last_command.clone()))));
            }
            Ok((None, None))
        }
        _ => Ok((None, None)),
    }
}

/// Handle window resize events
fn handle_resize(
    width: u16,
    height: u16,
    ui_state: &UiState,
    indexes_to_display: &[CommandIndex],
) -> UiState {
    let new_height = height.saturating_sub(2);
    let mut ui_state = ui_state.clone();
    let mut new_viewport = ViewportState {
        width,
        height: new_height,
        offset: ui_state.viewport.offset,
    };

    // If growing taller, try to show more items above current selection
    match new_height.cmp(&ui_state.viewport.height) {
        std::cmp::Ordering::Greater if new_viewport.offset > 0 => {
            let height_increase = new_height - new_viewport.height;
            new_viewport.offset = new_viewport.offset.saturating_sub(height_increase as usize);
        }
        std::cmp::Ordering::Less
            if ui_state.selected_index >= new_viewport.offset + new_height as usize =>
        {
            new_viewport.offset = ui_state
                .selected_index
                .saturating_sub(new_height as usize - 1);

            if new_viewport.offset + new_height as usize > indexes_to_display.len() {
                new_viewport.offset = indexes_to_display.len().saturating_sub(new_height as usize);
            }
        }
        _ => {}
    }

    ui_state.viewport = new_viewport;
    ui_state
}

fn handle_mouse_event(
    kind: MouseEventKind,
    previous_mouse_down_row: Option<u16>,
    mouse_down_row: u16,
    ui_state: &UiState,
    indexes_to_display: &[CommandIndex],
    last_command: &Option<&CommandExecutionTemplate>,
) -> (
    Option<UiState>,       // UIState if it needs redrawing
    Option<CommandChoice>, // if command selected, this is populated
    Option<u16>,           // clicked row
) {
    match kind {
        MouseEventKind::Down(button) => {
            if button == MouseButton::Left {
                return (None, None, Some(mouse_down_row));
            }

            (None, None, None)
        }
        MouseEventKind::Up(button) => {
            if button == MouseButton::Left {
                if let Some(down_row) = previous_mouse_down_row {
                    if down_row == 0 {
                        // Click on header
                        return (None, None, None);
                    }

                    let clicked_index = (down_row - 1) as usize + ui_state.viewport.offset;

                    if clicked_index < indexes_to_display.len() {
                        let clicked_command = match indexes_to_display[clicked_index] {
                            Normal(i) => Some(CommandChoice::Index(i)),
                            CommandIndex::Rerun => (*last_command)
                                .map(|last_command| CommandChoice::Rerun(last_command.clone())),
                        };

                        return (None, clicked_command, Some(down_row));
                    }
                }
            }
            (None, None, None)
        }
        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
            let index_change_direction = if kind == MouseEventKind::ScrollDown {
                Down
            } else {
                Up
            };

            let new_state =
                move_selected_index(ui_state, indexes_to_display.len(), &index_change_direction);

            (Some(new_state), None, None)
        }
        _ => (None, None, None),
    }
}

/// Print the header for the command selection UI
fn print_header(ui_state: &UiState, command_display_count: usize) -> Result<()> {
    let mut stdout = stdout();
    let (width, _) = terminal::size()?;

    let left_padding_size = 2usize;

    let left_padding = " ".repeat(left_padding_size);

    let instructions = if ui_state.is_filtering {
        "<esc>: Stop Filtering".to_string()
    } else {
        format!(
            "/: Begin Filtering   |   {}/{}   |   q: Quit",
            pad_to_width_of(ui_state.selected_index + 1, command_display_count),
            command_display_count
        )
    };

    let right_padding = " ".repeat(width as usize - left_padding_size - instructions.len());

    queue!(
        stdout,
        MoveTo(0, 0),
        SetBackgroundColor(DarkGreen),
        Print(left_padding),
        Print(instructions),
        Print(right_padding),
        SetBackgroundColor(Reset),
        SetForegroundColor(Reset),
    )?;

    Ok(())
}

/// Pad a value to match the width of the largest value
fn pad_to_width_of<T: Display>(value: T, max_number: usize) -> String {
    let width = format!("{}", max_number).len();
    format!("{:>width$}", value.to_string())
}

/// Clear and write a command row in the selection UI
fn clear_and_write_command_row(
    row: u16,
    commands_to_display: &HashMap<CommandIndex, CommandForDisplay>,
    command_index: &CommandIndex,
    is_selected: bool,
    terminal_width: Option<u16>,
) -> Result<()> {
    let mut stdout = stdout();
    let terminal_width = terminal_width.unwrap_or_else(|| {
        let (width, _) = terminal::size().unwrap_or((0, 0));
        width
    });

    queue!(stdout, MoveTo(0, row), Clear(ClearType::CurrentLine))?;

    let index_as_string = pad_to_width_of(command_index, commands_to_display.len() + 1);
    let fw_index = format!("[{index_as_string}]");

    let command_definition = commands_to_display.get(command_index).unwrap();
    let content = format!("{fw_index} {command_definition}");

    let padding = if content.len() < (terminal_width as usize) {
        " ".repeat(terminal_width as usize - content.len())
    } else {
        "".to_string()
    };

    if is_selected {
        queue!(
            stdout,
            SetAttribute(Attribute::Bold),
            SetBackgroundColor(DarkBlue),
            SetForegroundColor(Yellow),
        )?;
    }

    let mut custom_background_color: Option<Color> = None;

    let mut custom_foreground_color: Option<Color> = None;
    if let CommandForDisplay::Normal(cd) = command_definition {
        if let Some(b_c) = cd.background_color()? {
            custom_background_color = Some(b_c);
        }

        if let Some(fc) = cd.foreground_color()? {
            custom_foreground_color = Some(fc);
        }
    };

    if !is_selected {
        let background_color = custom_background_color.unwrap_or(Reset);

        let foreground_color = custom_foreground_color.unwrap_or(Reset);
        queue!(
            stdout,
            SetBackgroundColor(background_color),
            SetForegroundColor(foreground_color),
        )?;
    }

    queue!(stdout, Print(content), Print(padding),)?;

    queue!(
        stdout,
        SetAttribute(Attribute::Reset),
        SetBackgroundColor(Reset),
        SetForegroundColor(Reset),
    )?;

    Ok(())
}

/// Print all commands with the selected one highlighted
fn print_commands_with_selection(
    ui_state: &UiState,
    commands_to_display: &HashMap<CommandIndex, CommandForDisplay>,
    indexes_to_display: &[CommandIndex],
) -> Result<()> {
    let mut stdout = stdout();

    let viewport = &ui_state.viewport;

    let visible_commands = indexes_to_display
        .iter()
        .skip(viewport.offset)
        .take(viewport.height as usize);

    for (i, index) in visible_commands.enumerate() {
        let is_selected = i + viewport.offset == ui_state.selected_index;

        clear_and_write_command_row(
            i as u16 + 1,
            commands_to_display,
            index,
            is_selected,
            Some(viewport.width),
        )?;
        queue!(stdout, cursor::MoveToNextLine(1))?;
    }

    Ok(())
}

/// Move the selected index in the given direction
fn move_selected_index(
    ui_state: &UiState,
    commands_to_display_length: usize,
    direction: &CycleDirection,
) -> UiState {
    if commands_to_display_length == 0 {
        return ui_state.clone();
    }

    let mut new_index = ui_state.selected_index;
    let mut ui_state = ui_state.clone();

    match direction {
        Up => {
            if new_index == 0 {
                new_index = commands_to_display_length - 1;
                ui_state.viewport.offset =
                    new_index.saturating_sub(ui_state.viewport.height as usize - 1);
            } else {
                new_index -= 1;
                if new_index < ui_state.viewport.offset {
                    ui_state.viewport.offset = new_index;
                }
            }
        }
        Down => {
            new_index = (new_index + 1) % commands_to_display_length;
            if new_index < ui_state.selected_index {
                ui_state.viewport.offset = 0;
            } else if new_index >= ui_state.viewport.offset + ui_state.viewport.height as usize {
                ui_state.viewport.offset = new_index - ui_state.viewport.height as usize + 1;
            }
        }
    }

    ui_state.selected_index = new_index;
    ui_state
}

/// Filter the displayed command indexes based on a predicate
fn filter_displayed_indexes(
    matcher: &SkimMatcherV2,
    command_lookup: &HashMap<CommandIndex, CommandForDisplay>,
    predicate: &str,
) -> Vec<CommandIndex> {
    let predicate_index = predicate.parse::<usize>().ok();

    let mut filtered: Vec<CommandIndex> = command_lookup
        .iter()
        .filter_map(|(i, command_for_display)| {
            let command_description = command_for_display.to_string();

            if let Some(pred_idx) = predicate_index {
                // Index-based filtering
                i.to_string()
                    .contains(&pred_idx.to_string())
                    .then_some(i.clone())
            } else {
                // Fuzzy name-based filtering
                matcher
                    .fuzzy_match(&command_description, predicate)
                    .map(|_| i.clone())
            }
        })
        .collect();

    filtered.sort_by(|k1, k2| k1.compare(k2));

    filtered
}
