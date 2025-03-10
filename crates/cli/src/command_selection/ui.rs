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
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{cursor, event, execute, queue, terminal, ExecutableCommand};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use super::types::{
    CommandChoice, CommandForDisplay, CommandIndex, CycleDirection, DisplayMode, ViewportState,
};
use super::colors::CommandDefinitionColor;
use super::LAST_COMMAND_OPTION;
use crate::command_selection::types::CommandIndex::Normal;
use crate::command_selection::types::CycleDirection::{Down, Up};
use rust_cuts_core::command_definitions::{CommandDefinition, CommandExecutionTemplate};
use rust_cuts_core::error::{Error, Result};

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // Disable raw mode on drop
        let _ = disable_raw_mode();
        let mut stdout = stdout();
        let _ = stdout.execute(DisableMouseCapture);
    }
}


/// Prompts the user to choose a command from the list
pub fn prompt_for_command_choice(
    command_definitions: &[CommandDefinition],
    last_command: Option<&CommandExecutionTemplate>,
) -> Result<CommandChoice> {
    let mut stdout = stdout();

    let mut selected_index: usize = 0;
    enable_raw_mode()?;

    let _raw_mode_guard = RawModeGuard; // When this goes out of scope, raw mode and mouse capture is disabled
    stdout.execute(event::EnableMouseCapture)?;

    let mut should_reprint = true;
    let mut typed_index = String::new();
    let mut filter_text = String::new();
    let mut display_mode = DisplayMode {
        is_filtering: false,
    };

    let mut command_display: HashMap<CommandIndex, CommandForDisplay> = command_definitions
        .iter()
        .enumerate()
        .map(|(i, cd)| {
            (
                CommandIndex::Normal(i),
                CommandForDisplay::Normal(cd.clone()),
            )
        })
        .collect();

    if let Some(lc) = last_command {
        command_display.insert(CommandIndex::Rerun, CommandForDisplay::Rerun(lc.clone()));
    }

    let mut indexes_to_display = filter_displayed_indexes(&command_display, &filter_text);

    let mut down_row: Option<u16> = None;
    let mut index_change_direction: Option<CycleDirection> = None;

    let (width, height) = terminal::size()?;

    let mut viewport = ViewportState {
        offset: 0,
        height: height.saturating_sub(2), // Subtract 2 for header and filter line
        width,
    };

    loop {
        if should_reprint {
            let indexes_before = indexes_to_display.clone();
            indexes_to_display = filter_displayed_indexes(&command_display, &filter_text);

            if indexes_before == indexes_to_display {
                selected_index = typed_index.parse::<usize>().unwrap_or(0);
            } else {
                (selected_index, _) = move_selected_index(
                    selected_index,
                    &mut viewport,
                    indexes_to_display.len(),
                    None,
                );
                typed_index = selected_index.to_string();
            }

            queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

            print_header(&display_mode, selected_index, indexes_to_display.len())?;

            if indexes_to_display.is_empty() {
                queue!(
                    stdout,
                    SetForegroundColor(Color::Red),
                    Print("No matching commands!".to_string()),
                    SetAttribute(Attribute::Reset),
                    cursor::MoveToNextLine(1)
                )?;
            } else {
                print_commands_with_selection(
                    &command_display,
                    &indexes_to_display,
                    selected_index,
                    &viewport,
                )?;
            }

            if display_mode.is_filtering {
                queue!(
                    stdout,
                    SetAttribute(Attribute::Bold),
                    Print(format!("Filter: {filter_text}")),
                    SetAttribute(Attribute::Reset)
                )?;
            }

            stdout.flush()?;
            should_reprint = false;
        }

        if event::poll(Duration::from_millis(500))? {
            match event::read()? {
                Event::Mouse(MouseEvent {
                                 kind,
                                 row,
                                 modifiers,
                                 ..
                             }) => {
                    if modifiers == KeyModifiers::NONE {
                        match kind {
                            MouseEventKind::Down(button) => {
                                if button == MouseButton::Left {
                                    down_row = Some(row);
                                }
                            }
                            MouseEventKind::Up(button) => {
                                if button == MouseButton::Left {
                                    if let Some(down_row) = down_row {
                                        let clicked_index =
                                            (down_row - 1) as usize + viewport.offset;

                                        if clicked_index < indexes_to_display.len() {
                                            clear_and_write_command_row(
                                                selected_index as u16 + 1,
                                                &command_display,
                                                &indexes_to_display[selected_index],
                                                false,
                                                None,
                                            )?;

                                            clear_and_write_command_row(
                                                down_row,
                                                &command_display,
                                                &indexes_to_display[clicked_index],
                                                true,
                                                None,
                                            )?;

                                            selected_index = clicked_index;
                                            queue!(
                                                stdout,
                                                MoveTo(0, indexes_to_display.len() as u16 + 1)
                                            )?;
                                            match indexes_to_display[clicked_index] {
                                                Normal(i) => return Ok(CommandChoice::Index(i)),
                                                CommandIndex::Rerun => {
                                                    if let Some(last_command) = last_command {
                                                        return Ok(CommandChoice::Rerun(
                                                            last_command.clone(),
                                                        ));
                                                    };
                                                }
                                            }
                                        }
                                    }
                                    down_row = None;
                                }
                            }
                            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                                index_change_direction = if kind == MouseEventKind::ScrollDown {
                                    Some(Down)
                                } else {
                                    Some(Up)
                                };
                            }
                            _ => {}
                        }
                    }
                }
                Event::Key(key_event) => {
                    handle_key_event(
                        key_event,
                        &mut index_change_direction,
                        &mut display_mode,
                        &mut filter_text,
                        &mut should_reprint,
                        &indexes_to_display,
                        selected_index,
                        last_command,
                    )?;
                }
                Event::Resize(width, height) => {
                    handle_resize(
                        width,
                        height,
                        &mut viewport,
                        selected_index,
                        &indexes_to_display,
                        &mut should_reprint,
                    );
                }
                Event::FocusGained => {}
                Event::FocusLost => {}
                Event::Paste(_) => {}
            }

            if let Some(direction) = index_change_direction.take() {
                handle_index_change(
                    direction,
                    &mut selected_index,
                    &mut viewport,
                    &indexes_to_display,
                    &command_display,
                    &mut typed_index,
                    &mut should_reprint,
                    &display_mode,
                )?;
            }
        }
    }
}

/// Handle keyboard events in the command selection UI
fn handle_key_event(
    key_event: crossterm::event::KeyEvent,
    index_change_direction: &mut Option<CycleDirection>,
    display_mode: &mut DisplayMode,
    filter_text: &mut String,
    should_reprint: &mut bool,
    indexes_to_display: &[CommandIndex],
    selected_index: usize,
    last_command: Option<&CommandExecutionTemplate>,
) -> Result<Option<CommandChoice>> {
    match key_event.code {
        KeyCode::Up | KeyCode::Down => {
            *index_change_direction = if key_event.code == KeyCode::Up {
                Some(Up)
            } else {
                Some(Down)
            };
            Ok(None)
        }
        KeyCode::Enter => {
            if let Some(command_index) = indexes_to_display.get(selected_index) {
                match command_index {
                    Normal(i) => return Ok(Some(CommandChoice::Index(*i))),
                    CommandIndex::Rerun => {
                        if let Some(last_command) = last_command {
                            return Ok(Some(CommandChoice::Rerun(last_command.clone())));
                        };
                    }
                }
            } else {
                execute!(stdout(), Print("\x07"))?;
            }
            Ok(None)
        }
        KeyCode::Backspace => {
            if filter_text.pop().is_some() {
                *should_reprint = true;
            }
            Ok(None)
        }
        KeyCode::Char('c')
        if key_event
            .modifiers
            .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                Ok(Some(CommandChoice::Quit))
            }
        KeyCode::Char(c) if display_mode.is_filtering => {
            filter_text.push(c);
            *should_reprint = true;
            Ok(None)
        }
        KeyCode::Esc if display_mode.is_filtering => {
            display_mode.is_filtering = false;
            *should_reprint = true;
            *filter_text = "".to_string();
            Ok(None)
        }
        KeyCode::Char('/') => {
            display_mode.is_filtering = true;
            *should_reprint = true;
            Ok(None)
        }
        KeyCode::Char('q') => {
            Ok(Some(CommandChoice::Quit))
        }
        KeyCode::Char(LAST_COMMAND_OPTION) => {
            if let Some(last_command) = last_command {
                return Ok(Some(CommandChoice::Rerun(last_command.clone())));
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Handle window resize events
fn handle_resize(
    width: u16,
    height: u16,
    viewport: &mut ViewportState,
    selected_index: usize,
    indexes_to_display: &[CommandIndex],
    should_reprint: &mut bool,
) {
    let new_height = height.saturating_sub(2);
    viewport.width = width;

    // If growing taller, try to show more items above current selection
    match new_height.cmp(&viewport.height) {
        std::cmp::Ordering::Greater if viewport.offset > 0 => {
            let height_increase = new_height - viewport.height;
            viewport.offset = viewport.offset.saturating_sub(height_increase as usize);
        }
        std::cmp::Ordering::Less if selected_index >= viewport.offset + new_height as usize => {
            viewport.offset = selected_index.saturating_sub(new_height as usize - 1);

            if viewport.offset + new_height as usize > indexes_to_display.len() {
                viewport.offset = indexes_to_display.len().saturating_sub(new_height as usize);
            }
        }
        _ => {}
    }

    viewport.height = new_height;
    *should_reprint = true;
}

/// Handle changes to the selected index
fn handle_index_change(
    direction: CycleDirection,
    selected_index: &mut usize,
    viewport: &mut ViewportState,
    indexes_to_display: &[CommandIndex],
    command_display: &HashMap<CommandIndex, CommandForDisplay>,
    typed_index: &mut String,
    should_reprint: &mut bool,
    display_mode: &DisplayMode,
) -> Result<()> {
    let (new_index, viewport_changed) = move_selected_index(
        *selected_index,
        viewport,
        indexes_to_display.len(),
        Some(&direction),
    );

    if viewport_changed {
        *should_reprint = true;
    } else {
        print_header(display_mode, new_index, indexes_to_display.len())?;

        // Calculate visible row positions relative to viewport
        let old_row = (*selected_index - viewport.offset) as u16 + 1;
        let new_row = (new_index - viewport.offset) as u16 + 1;

        // Only try to update individual rows if they're both visible
        if old_row > 0
            && old_row <= viewport.height
            && new_row > 0
            && new_row <= viewport.height
        {
            clear_and_write_command_row(
                old_row,
                command_display,
                &indexes_to_display[*selected_index],
                false,
                None,
            )?;

            clear_and_write_command_row(
                new_row,
                command_display,
                &indexes_to_display[new_index],
                true,
                None,
            )?;
        } else {
            // If either row isn't visible, we need a full redraw
            *should_reprint = true;
        }
    }

    *selected_index = new_index;
    *typed_index = selected_index.to_string();

    Ok(())
}

/// Print the header for the command selection UI
fn print_header(
    header_mode: &DisplayMode,
    selected_index: usize,
    command_display_count: usize,
) -> Result<()> {
    let mut stdout = stdout();
    let (width, _) = terminal::size()?;

    let left_padding_size = 2usize;

    let left_padding = " ".repeat(left_padding_size);

    let instructions = if header_mode.is_filtering {
        "<esc>: Stop Filtering".to_string()
    } else {
        format!(
            "/: Begin Filtering   |   {}/{}   |   q: Quit",
            pad_to_width_of(selected_index + 1, command_display_count),
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
    stdout.flush()?;

    Ok(())
}

/// Print all commands with the selected one highlighted
fn print_commands_with_selection(
    commands_to_display: &HashMap<CommandIndex, CommandForDisplay>,
    indexes_to_display: &[CommandIndex],
    selected_index: usize,
    viewport: &ViewportState,
) -> Result<()> {
    let mut stdout = stdout();

    let visible_commands = indexes_to_display
        .iter()
        .skip(viewport.offset)
        .take(viewport.height as usize);

    for (i, index) in visible_commands.enumerate() {
        let is_selected = i + viewport.offset == selected_index;

        clear_and_write_command_row(
            i as u16 + 1,
            commands_to_display,
            index,
            is_selected,
            Some(viewport.width),
        )?;
        queue!(stdout, cursor::MoveToNextLine(1))?;
    }

    if let Err(e) = stdout.flush() {
        return Err(Error::Stdio(e));
    }

    Ok(())
}

/// Move the selected index in the given direction
fn move_selected_index(
    current_index: usize,
    viewport: &mut ViewportState,
    commands_to_display_length: usize,
    direction: Option<&CycleDirection>,
) -> (usize, bool) {
    if commands_to_display_length == 0 {
        return (0, false);
    }

    let mut new_index = current_index;
    let mut viewport_changed = false;

    match direction {
        Some(Up) => {
            if new_index == 0 {
                new_index = commands_to_display_length - 1;
                viewport.offset = new_index.saturating_sub(viewport.height as usize - 1);
                viewport_changed = true;
            } else {
                new_index -= 1;
                if new_index < viewport.offset {
                    viewport.offset = new_index;
                    viewport_changed = true;
                }
            }
        }
        Some(Down) => {
            new_index = (new_index + 1) % commands_to_display_length;
            if new_index < current_index {
                viewport.offset = 0;
                viewport_changed = true;
            } else if new_index >= viewport.offset + viewport.height as usize {
                viewport.offset = new_index - viewport.height as usize + 1;
                viewport_changed = true;
            }
        }
        None => {}
    }

    (new_index, viewport_changed)
}

/// Filter the displayed command indexes based on a predicate
fn filter_displayed_indexes(
    command_lookup: &HashMap<CommandIndex, CommandForDisplay>,
    predicate: &str,
) -> Vec<CommandIndex> {
    let matcher = SkimMatcherV2::default();
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
