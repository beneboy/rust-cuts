use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::{stdin, stdout, Write};
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
use crossterm::{cursor, event, queue, terminal, ExecutableCommand};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::command_definitions::{CommandDefinition, CommandExecutionTemplate};
use crate::command_selection::CommandIndex::Normal;
use crate::command_selection::CycleDirection::{Down, Up};
use crate::error::{Error, Result};
use crate::LAST_COMMAND_OPTION;

pub enum CommandChoice {
    Index(usize),
    Rerun(CommandExecutionTemplate),
    Quit,
}

pub enum RunChoice {
    Yes,
    No,
    ChangeParams,
}

struct DisplayMode {
    is_filtering: bool,
}

pub fn prompt_value(variable_name: &str, default_value: Option<&String>) -> Result<String> {
    loop {
        if default_value.is_some() {
            print!(
                "Please give value for `{variable_name}` [{}]: ",
                default_value.as_ref().unwrap()
            );
        } else {
            print!("Please give value for `{variable_name}`: ");
        }
        stdout().flush()?;

        let mut input = String::new();
        stdin().read_line(&mut input)?;

        let read_value = input.trim().to_string();

        if !read_value.is_empty() {
            return Ok(read_value);
        }

        if let Some(default_value) = default_value {
            return Ok((*default_value).to_string());
        }
    }
}

pub fn confirm_command_should_run(has_params: bool) -> Result<RunChoice> {
    loop {
        let prompt_change_params = if has_params {
            "/[c]hange parameters"
        } else {
            ""
        };

        print!("Are you sure you want to run? ([Y]es/[n]o{prompt_change_params}): ");
        stdout().flush()?;

        // Read user input
        let mut input = String::new();
        stdin().read_line(&mut input)?;

        let lowercase_input = input.trim().to_lowercase();

        if lowercase_input.as_str() == "y" || lowercase_input.is_empty() {
            return Ok(RunChoice::Yes);
        }

        if lowercase_input.as_str() == "n" {
            return Ok(RunChoice::No);
        }

        if has_params && lowercase_input.as_str() == "c" {
            return Ok(RunChoice::ChangeParams);
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum CommandIndex {
    Normal(usize),
    Rerun,
}

impl Display for CommandIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandIndex::Normal(i) => f.write_str(format!("{i}").as_str()),
            CommandIndex::Rerun => f.write_str("r"),
        }
    }
}

fn print_header(header_mode: &DisplayMode) -> Result<()> {
    let mut stdout = stdout();
    let (width, _) = terminal::size()?;

    let left_padding_size = 2usize;

    let left_padding = " ".repeat(left_padding_size);

    let instructions = if header_mode.is_filtering {
        "<esc>: Stop Filtering"
    } else {
        "/: Begin Filtering   |   q: Quit"
    };

    let right_padding = " ".repeat(width as usize - left_padding_size - instructions.len());

    queue!(
        stdout,
        SetBackgroundColor(DarkGreen),
        Print(left_padding),
        Print(instructions),
        Print(right_padding),
        SetBackgroundColor(Reset),
        SetForegroundColor(Reset),
    )?;

    Ok(())
}

fn clear_and_write_command_row(
    row: u16,
    commands_to_display: &HashMap<CommandIndex, String>,
    command_index: &CommandIndex,
    is_selected: bool,
    terminal_width: Option<u16>,
) -> Result<()> {
    let mut stdout = stdout();
    let terminal_width = terminal_width.unwrap_or_else(|| {
        let (width, _) = terminal::size().unwrap_or((0, 0));
        width
    });

    let max_digits = format!("{highest_index}", highest_index = commands_to_display.len()).len();

    queue!(stdout, MoveTo(0, row), Clear(ClearType::CurrentLine))?;

    let index_as_string = command_index.to_string();
    let fw_index = format!("[{index_as_string:>max_digits$}] ");

    let command_definition = commands_to_display.get(command_index).unwrap();
    let content = format!("{fw_index} {command_definition}");
    let padding = " ".repeat(terminal_width as usize - content.len());

    if is_selected {
        queue!(
            stdout,
            SetAttribute(Attribute::Bold),
            SetBackgroundColor(DarkBlue),
            SetForegroundColor(Yellow),
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

fn print_commands_with_selection(
    commands_to_display: &HashMap<CommandIndex, String>,
    indexes_to_display: &[CommandIndex],
    selected_index: usize,
) -> Result<()> {
    let mut stdout = stdout();

    let (width, _) = terminal::size()?;

    for (i, index) in indexes_to_display.iter().enumerate() {
        let is_selected = i == selected_index;
        clear_and_write_command_row(
            i as u16 + 1,
            commands_to_display,
            index,
            is_selected,
            Some(width),
        )?;
        queue!(stdout, cursor::MoveToNextLine(1))?;
    }

    if let Err(e) = stdout.flush() {
        return Err(Error::Stdio(e));
    }

    Ok(())
}

enum CycleDirection {
    Up,
    Down,
}

fn move_selected_index(
    current_index: usize,
    commands_to_display_length: usize,
    direction: Option<&CycleDirection>,
) -> usize {
    if commands_to_display_length == 0 {
        return 0;
    }

    let mut new_index: usize = current_index;

    if new_index >= commands_to_display_length {
        new_index = commands_to_display_length - 1
    }

    match direction {
        Some(Up) => {
            if new_index == 0 {
                new_index = commands_to_display_length - 1
            } else {
                new_index -= 1
            }
        }
        Some(Down) => new_index += 1,
        None => {}
    }

    new_index % commands_to_display_length
}

fn filter_displayed_indexes(
    command_lookup: &HashMap<CommandIndex, String>,
    predicate: &str,
) -> Vec<CommandIndex> {
    let matcher = SkimMatcherV2::default();
    let predicate_index = predicate.parse::<usize>().ok();

    let mut filtered: Vec<CommandIndex> = command_lookup
        .iter()
        .filter_map(|(i, command_description)| {
            if let Some(pred_idx) = predicate_index {
                // Index-based filtering
                i.to_string()
                    .contains(&pred_idx.to_string())
                    .then_some(i.clone())
            } else {
                // Fuzzy name-based filtering
                matcher
                    .fuzzy_match(command_description, predicate)
                    .map(|_| i.clone())
            }
        })
        .collect();

    filtered.sort_by(|k1, k2| match (k1, k2) {
        (Normal(i1), Normal(i2)) => i1.cmp(i2),
        (_, Normal(_)) => Ordering::Greater,
        (Normal(_), _) => Ordering::Less,
        _ => Ordering::Equal,
    });

    filtered
}

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

    let mut command_display: HashMap<CommandIndex, String> = command_definitions
        .iter()
        .enumerate()
        .map(|(i, cd)| (CommandIndex::Normal(i), cd.to_string()))
        .collect();

    if let Some(lc) = last_command {
        command_display.insert(CommandIndex::Rerun, lc.to_string());
    }

    let mut indexes_to_display = filter_displayed_indexes(&command_display, &filter_text);

    let mut down_row: Option<u16> = None;
    let mut index_change_direction: Option<CycleDirection> = None;

    loop {
        if should_reprint {
            let indexes_before = indexes_to_display.clone();
            indexes_to_display = filter_displayed_indexes(&command_display, &filter_text);

            if indexes_before == indexes_to_display {
                selected_index = typed_index.parse::<usize>().unwrap_or(0);
            } else {
                selected_index =
                    move_selected_index(selected_index, indexes_to_display.len(), None);
                typed_index = selected_index.to_string();
            }

            queue!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

            print_header(&display_mode)?;

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
                                        let clicked_index = (down_row - 1) as usize;

                                        if clicked_index < indexes_to_display.len() {
                                            clear_and_write_command_row(
                                                down_row,
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
                    match key_event.code {
                        KeyCode::Up | KeyCode::Down => {
                            index_change_direction = if key_event.code == KeyCode::Up {
                                Some(Up)
                            } else {
                                Some(Down)
                            };
                        }
                        KeyCode::Enter => match indexes_to_display[selected_index] {
                            Normal(i) => return Ok(CommandChoice::Index(i)),
                            CommandIndex::Rerun => {
                                if let Some(last_command) = last_command {
                                    return Ok(CommandChoice::Rerun(last_command.clone()));
                                };
                            }
                        },
                        KeyCode::Backspace => {
                            if filter_text.pop().is_some() {
                                should_reprint = true;
                            }
                        }
                        KeyCode::Char('c')
                            if key_event
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            return Ok(CommandChoice::Quit);
                        }
                        KeyCode::Char(c) if display_mode.is_filtering => {
                            filter_text.push(c);
                            should_reprint = true;
                        }
                        KeyCode::Esc if display_mode.is_filtering => {
                            display_mode.is_filtering = false;
                            should_reprint = true;
                            filter_text = "".to_string();
                        }
                        // KeyCode::Char(d) if d.is_ascii_digit() && !filter_mode => {
                        //     typed_index.push(d);
                        //     should_reprint = true;
                        // }
                        KeyCode::Char('/') => {
                            display_mode.is_filtering = true;
                            should_reprint = true;
                        }
                        KeyCode::Char('q') => {
                            return Ok(CommandChoice::Quit);
                        }
                        KeyCode::Char(LAST_COMMAND_OPTION) => {
                            if let Some(last_command) = last_command {
                                return Ok(CommandChoice::Rerun(last_command.clone()));
                            }
                        }
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {
                    should_reprint = true;
                }
                Event::FocusGained => {}
                Event::FocusLost => {}
                Event::Paste(_) => {}
            }

            match index_change_direction {
                None => {}
                Some(d) => {
                    clear_and_write_command_row(
                        selected_index as u16 + 1,
                        &command_display,
                        &indexes_to_display[selected_index],
                        false,
                        None,
                    )?;

                    selected_index =
                        move_selected_index(selected_index, indexes_to_display.len(), Some(&d));

                    clear_and_write_command_row(
                        selected_index as u16 + 1,
                        &command_display,
                        &indexes_to_display[selected_index],
                        true,
                        None,
                    )?;
                    typed_index = selected_index.to_string();
                    index_change_direction = None;
                }
            }
        }
    }
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // Disable raw mode on drop
        let _ = disable_raw_mode();
        let mut stdout = stdout();
        let _ = stdout.execute(DisableMouseCapture);
    }
}
