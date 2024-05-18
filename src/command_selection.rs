use std::fmt::Display;
use std::io;
use std::io::Write;

use crate::command_definitions::{CommandDefinition, CommandExecutionTemplate};
use crate::error::Result;
use crate::interpolation::{get_templates, interpolate_command};
use crate::LAST_COMMAND_OPTION;

pub enum CommandSelectionResult {
    Index(usize),
    Rerun(CommandExecutionTemplate),
    Quit,
}

pub enum CommandRunResult {
    Yes,
    No,
    ChangeParams
}

pub fn read_option_input(
    max: usize,
    last_command_parameters: &Option<CommandExecutionTemplate>
) -> Result<CommandSelectionResult> {
    loop {
        let rerun_text = match last_command_parameters.is_some() {
            true => format!(", or `{}` to re-run last", LAST_COMMAND_OPTION),
            false => "".to_string()
        };
        // Prompt the user for input
        print!("Enter an option (0-{}{}. Quit with `q`): ", max - 1, rerun_text);
        io::stdout().flush()?;

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        input = input.trim().to_lowercase();

        if input == "q" {
            return Ok(CommandSelectionResult::Quit);
        }

        if input == "r" {
            match last_command_parameters {
                None => continue,  // this shouldn't happen, but it's not a fatal error
                Some(last_command_parameters) => {
                    return Ok(CommandSelectionResult::Rerun(last_command_parameters.clone()));
                }
            }
        }

        // Parse input as usize
        match input.parse::<usize>() {
            Ok(index) => {
                if index < max {
                    return Ok(CommandSelectionResult::Index(index));
                } else {
                    println!("Index must be between 0 and {}", max - 1);
                }
            }
            Err(_) => {
                println!("Invalid input. Please enter a valid index.");
            }
        }
    }
}

pub fn prompt_value(variable_name: &str, default_value: &Option<&String>) -> Result<String> {
    loop {
        if default_value.is_some() {
            print!(
                "Please give value for `{}` [{}]: ", variable_name, default_value.as_ref().unwrap()
            );
        } else {
            print!("Please give value for `{}`: ", variable_name);
        }
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let read_value = input.trim().to_string();

        if !read_value.is_empty() {
            return Ok(read_value);
        }

        if let Some(default_value) = default_value {
            return Ok(default_value.to_string())
        }
    }
}

pub fn confirm_command_should_run(has_params: bool) -> Result<CommandRunResult> {
    loop {
        let prompt_change_params = if has_params {
            "/[c]hange parameters"
        } else {
            ""
        };

        print!("Are you sure you want to run? ([Y]es/[n]o{}): ", prompt_change_params);
        io::stdout().flush()?;

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let lowercase_input = input.trim().to_lowercase();

        if lowercase_input.as_str() == "y" || lowercase_input.is_empty() {
            return Ok(CommandRunResult::Yes);
        }

        if lowercase_input.as_str() == "n" {
            return Ok(CommandRunResult::No);
        }

        if has_params && lowercase_input.as_str() == "c" {
            return Ok(CommandRunResult::ChangeParams);
        }
    }
}

pub fn prompt_for_command_selection(
    command_definitions: &[CommandDefinition],
    last_command: &Option<CommandExecutionTemplate>
) -> Result<CommandSelectionResult> {
    for (index, command_def) in command_definitions.iter().enumerate() {
        println!("{}", format_command_def(&index, &command_def));
    }

    if let Some(last_command) = last_command.clone() {
        let template_context = last_command.template_context;
        let interpolated_last_command = interpolate_command(
            &template_context, &get_templates(&last_command.command)?
        )?;
        println!(
            "{}",
            format_command_def(&LAST_COMMAND_OPTION, &interpolated_last_command.join(" "))
        );
    }

    read_option_input(command_definitions.len(), last_command)
}


fn format_command_def(option: &impl Display, command_label: &impl Display) -> String {
    format!("[{option}]: {command_label}")
}
