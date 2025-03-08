use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
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
use crossterm::{cursor, event, execute, queue, terminal, ExecutableCommand};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use itertools::Itertools;

use crate::command_selection::CommandIndex::Normal;
use crate::command_selection::CycleDirection::{Down, Up};
use crate::LAST_COMMAND_OPTION;
use rust_cuts_core::command_definitions::{
    ColorDefinition, CommandDefinition, CommandExecutionTemplate, ParameterDefinition,
};
use rust_cuts_core::error::{Error, Result};

pub enum CommandChoice {
    Index(usize),
    CommandId(String),
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

struct ViewportState {
    offset: usize,
    height: u16,
    width: u16,
}

trait AsTermColor {
    fn as_crossterm_color(&self) -> Result<Option<Color>>;
}

impl AsTermColor for ColorDefinition {
    fn as_crossterm_color(&self) -> Result<Option<Color>> {
        let defined_count = [self.rgb.is_some(), self.ansi.is_some(), self.name.is_some()]
            .iter()
            .filter(|&&x| x)
            .count();

        // Error if more than one field is defined
        if defined_count > 1 {
            return Err(Error::MultipleColorTypes);
        }

        // Convert to crossterm Color
        Ok(match (self.rgb, self.ansi, &self.name) {
            (Some((r, g, b)), None, None) => Some(Color::Rgb { r, g, b }),
            (None, Some(ansi), None) => Some(Color::AnsiValue(ansi)),
            (None, None, Some(name)) => Some(match name.to_lowercase().as_str() {
                "black" => Color::Black,
                "darkgrey" => Color::DarkGrey,
                "red" => Color::Red,
                "darkred" => Color::DarkRed,
                "green" => Color::Green,
                "darkgreen" => Color::DarkGreen,
                "yellow" => Color::Yellow,
                "darkyellow" => Color::DarkYellow,
                "blue" => Color::Blue,
                "darkblue" => Color::DarkBlue,
                "magenta" => Color::Magenta,
                "darkmagenta" => Color::DarkMagenta,
                "cyan" => Color::Cyan,
                "darkcyan" => Color::DarkCyan,
                "white" => Color::White,
                "grey" => Color::Grey,
                _ => return Err(Error::UnknownColorName(name.to_string())),
            }),
            (None, None, None) => None,
            _ => unreachable!(), // This case is prevented by the earlier check
        })
    }
}

fn color_from_metadata_attribute(
    color_definition: &Option<ColorDefinition>,
) -> Result<Option<Color>> {
    match color_definition {
        None => Ok(None),
        Some(color_definition) => color_definition.as_crossterm_color(),
    }
}

trait CommandDefinitionColor {
    fn foreground_color(&self) -> Result<Option<Color>>;
    fn background_color(&self) -> Result<Option<Color>>;
}

impl CommandDefinitionColor for CommandDefinition {
    fn foreground_color(&self) -> Result<Option<Color>> {
        if let Some(metadata) = &self.metadata {
            color_from_metadata_attribute(&metadata.foreground_color)
        } else {
            Ok(None)
        }
    }

    fn background_color(&self) -> Result<Option<Color>> {
        if let Some(metadata) = &self.metadata {
            color_from_metadata_attribute(&metadata.background_color)
        } else {
            Ok(None)
        }
    }
}

pub fn prompt_value(
    variable_name: &str,
    parameter_definition: Option<&ParameterDefinition>,
    previous_default: Option<String>,
) -> Result<String> {
    loop {
        // Determine what to display in the prompt
        let display_default = previous_default
            .as_ref()
            .or_else(|| parameter_definition.and_then(|def| def.default.as_ref()));

        let prompt_base = if let Some(param_def) = parameter_definition {
            format!("Value for {}", param_def)
        } else {
            format!("Value for `{variable_name}`")
        };

        if let Some(default) = &display_default {
            print!("{} [{}]: ", prompt_base, default);
        } else {
            print!("{}: ", prompt_base);
        }

        stdout().flush()?;

        // Read user input
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        let read_value = input.trim().to_string();

        // Return user input if not empty, otherwise return default
        if !read_value.is_empty() {
            return Ok(read_value);
        }

        // Return the previous_default or parameter default if available
        if let Some(default) = previous_default {
            return Ok(default);
        } else if let Some(param_def) = parameter_definition {
            if let Some(default) = &param_def.default {
                return Ok(default.clone());
            }
        }

        // No input and no default - loop again
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
            CommandIndex::Normal(i) => f.write_str(format!("{}", i + 1).as_str()),
            CommandIndex::Rerun => f.write_str("r"),
        }
    }
}

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

fn pad_to_width_of<T: Display>(value: T, max_number: usize) -> String {
    let width = format!("{}", max_number).len();
    format!("{:>width$}", value.to_string())
}

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

enum CycleDirection {
    Up,
    Down,
}

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

    filtered.sort_by(|k1, k2| match (k1, k2) {
        (Normal(i1), Normal(i2)) => i1.cmp(i2),
        (_, Normal(_)) => Ordering::Greater,
        (Normal(_), _) => Ordering::Less,
        _ => Ordering::Equal,
    });

    filtered
}

enum CommandForDisplay {
    Normal(CommandDefinition),
    Rerun(CommandExecutionTemplate),
}

impl Display for CommandForDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandForDisplay::Normal(n) => write!(f, "{}", n),
            CommandForDisplay::Rerun(r) => write!(f, "{}", r),
        }
    }
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
                    match key_event.code {
                        KeyCode::Up | KeyCode::Down => {
                            index_change_direction = if key_event.code == KeyCode::Up {
                                Some(Up)
                            } else {
                                Some(Down)
                            };
                        }
                        KeyCode::Enter => {
                            if let Some(command_index) = indexes_to_display.get(selected_index) {
                                match command_index {
                                    Normal(i) => return Ok(CommandChoice::Index(*i)),
                                    CommandIndex::Rerun => {
                                        if let Some(last_command) = last_command {
                                            return Ok(CommandChoice::Rerun(last_command.clone()));
                                        };
                                    }
                                }
                            } else {
                                execute!(stdout, Print("\x07"))?;
                            }
                        }
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
                Event::Resize(width, height) => {
                    let new_height = height.saturating_sub(2);
                    viewport.width = width;

                    // If growing taller, try to show more items above current selection
                    match new_height.cmp(&viewport.height) {
                        Ordering::Greater if viewport.offset > 0 => {
                            let height_increase = new_height - viewport.height;
                            viewport.offset =
                                viewport.offset.saturating_sub(height_increase as usize);
                        }
                        Ordering::Less
                            if selected_index >= viewport.offset + new_height as usize =>
                        {
                            viewport.offset =
                                selected_index.saturating_sub(new_height as usize - 1);

                            if viewport.offset + new_height as usize > indexes_to_display.len() {
                                viewport.offset =
                                    indexes_to_display.len().saturating_sub(new_height as usize);
                            }
                        }
                        _ => {}
                    }

                    viewport.height = new_height;
                    should_reprint = true;
                }
                Event::FocusGained => {}
                Event::FocusLost => {}
                Event::Paste(_) => {}
            }

            match index_change_direction {
                None => {}
                Some(d) => {
                    let (new_index, viewport_changed) = move_selected_index(
                        selected_index,
                        &mut viewport,
                        indexes_to_display.len(),
                        Some(&d),
                    );

                    if viewport_changed {
                        should_reprint = true;
                    } else {
                        print_header(&display_mode, new_index, indexes_to_display.len())?;

                        // Calculate visible row positions relative to viewport
                        let old_row = (selected_index - viewport.offset) as u16 + 1;
                        let new_row = (new_index - viewport.offset) as u16 + 1;

                        // Only try to update individual rows if they're both visible
                        if old_row > 0
                            && old_row <= viewport.height
                            && new_row > 0
                            && new_row <= viewport.height
                        {
                            clear_and_write_command_row(
                                old_row,
                                &command_display,
                                &indexes_to_display[selected_index],
                                false,
                                None,
                            )?;

                            clear_and_write_command_row(
                                new_row,
                                &command_display,
                                &indexes_to_display[new_index],
                                true,
                                None,
                            )?;
                        } else {
                            // If either row isn't visible, we need a full redraw
                            should_reprint = true;
                        }
                    }

                    selected_index = new_index;
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

pub fn fill_parameter_values(
    tokens: &HashSet<String>,
    parameter_definitions: &Option<HashMap<String, ParameterDefinition>>,
    existing_context: &Option<HashMap<String, ParameterDefinition>>,
) -> Result<Option<HashMap<String, ParameterDefinition>>> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut context: HashMap<String, ParameterDefinition> = HashMap::new();
    for key in tokens.iter().sorted() {
        // Get the previous context value if available
        let previous_context_param = existing_context
            .as_ref()
            .and_then(|ctx| ctx.get(key))
            .cloned();

        // Get the parameter definition if available
        let param_definition = parameter_definitions
            .as_ref()
            .and_then(|defs| defs.get(key))
            .cloned();

        // Determine the default value to display in the prompt
        let previous_default = previous_context_param
            .as_ref()
            .and_then(|param| param.default.clone())
            .or_else(|| param_definition.as_ref().and_then(|def| def.default.clone()));

        // Choose which parameter definition to display in the prompt
        let display_param = previous_context_param.as_ref().or(param_definition.as_ref());

        let prompted_value = prompt_value(key, display_param, previous_default)?;

        // Create or update the parameter definition
        let new_param = create_or_update_parameter(key, prompted_value, previous_context_param, param_definition);

        context.insert(key.clone(), new_param);
    }

    Ok(Some(context))
}

fn create_or_update_parameter(
    key: &str,
    value: String,
    previous_context_param: Option<ParameterDefinition>,
    parameter_definition: Option<ParameterDefinition>,
) -> ParameterDefinition {
    if let Some(mut param) = previous_context_param {
        // Use existing parameter, just update the default
        param.default = Some(value);
        param
    } else if let Some(mut def) = parameter_definition {
        // Use parameter definition from the command, update default
        // (this won't save back to original commands YAML)
        def.default = Some(value);
        def
    } else {
        // Both empty, create a new parameter definition
        ParameterDefinition {
            id: key.to_string(),
            default: Some(value),
            description: None,
        }
    }
}
