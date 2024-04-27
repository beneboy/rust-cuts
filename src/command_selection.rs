use std::io;
use std::io::Write;
use crate::LAST_COMMAND_OPTION;
use crate::command_definitions::LastCommandParameters;

pub enum CommandSelectionResult {
    Index(usize),
    Rerun(LastCommandParameters),
    Quit,
}

pub fn read_option_input(max: usize, last_command_parameters: Option<LastCommandParameters>) -> CommandSelectionResult {
    loop {
        let rerun_text = match last_command_parameters.is_some() {
            true => format!(", or `{}` to re-run last", LAST_COMMAND_OPTION),
            false => "".to_string()
        };
        // Prompt the user for input
        print!("Enter an option (0-{}{}. Quit with `q`): ", max - 1, rerun_text);
        io::stdout().flush().expect("Failed to flush stdout");

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        input = input.trim().to_lowercase();

        if input == "q" {
            return CommandSelectionResult::Quit;
        }

        if input == "r" {
            match last_command_parameters {
                None => continue,  // this shouldn't happen, but it's not a fatal error
                Some(last_command_parameters) => {
                    return CommandSelectionResult::Rerun(last_command_parameters);
                }
            }
        }

        // Parse input as usize
        match input.parse::<usize>() {
            Ok(index) => {
                if index < max {
                    return CommandSelectionResult::Index(index);
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

pub fn prompt_value(variable_name: &str, default_value: Option<String>) -> String {
    loop {
        if default_value.is_some() {
            print!("Please give value for `{}` [{}]: ", variable_name, default_value.as_ref().unwrap());
        } else {
            print!("Please give value for `{}`: ", variable_name);
        }
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        let read_value = input.trim().to_string();

        if read_value.is_empty() {
            if let Some(default_value) = default_value {
                return default_value;
            }
        } else {
            return read_value;
        }
    }
}

pub fn confirm_command_should_run() -> bool {
    loop {
        // Prompt the user for input
        print!("Are you sure you want to run? (Y/n): ");
        io::stdout().flush().expect("Failed to flush stdout");

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        let lowercase_input = input.trim().to_lowercase();

        if lowercase_input.as_str() == "y" || lowercase_input.is_empty() {
            return true;
        }

        if lowercase_input.as_str() == "n" {
            return false;
        }
    }
}
