use std::env;
use std::fmt::Display;
use std::process::{Command, ExitCode};

use clap::Parser;

mod cli_args;
mod command_definitions;
mod command_selection;
mod file_handling;
mod interpolation;
mod execution;
mod error;


use command_definitions::LastCommandParameters;
use command_selection::CommandSelectionResult::{Index, Quit, Rerun};
use crate::error::Result;


static DEFAULT_CONFIG_PATH: &str = "~/.rust-cuts/commands.yml";
static DEFAULT_LAST_COMMAND_PATH: &str = "~/.rust-cuts/last_command.yml";
static LAST_COMMAND_OPTION: &str = "r";

static DEFAULT_SHELL: &str = "/bin/bash";

fn get_config_path(config_path_arg: Option<String>) -> String {
    let config_path = &config_path_arg.unwrap_or(DEFAULT_CONFIG_PATH.to_string());
    shellexpand::tilde(config_path).to_string()
}

fn get_last_command_path(last_command_path_arg: Option<String>) -> String {
    let last_command_path = &last_command_path_arg.unwrap_or(DEFAULT_LAST_COMMAND_PATH.to_string());
    shellexpand::tilde(last_command_path).to_string()
}


fn format_command_def(option: &impl Display, command_label: &impl Display) -> String {
    format!("[{option}]: {command_label}")
}

fn execute() -> Result<()> {
    let args = cli_args::Args::parse();

    let shell = match env::var("SHELL") {
        Ok(shell) => { shell }
        Err(_) => { DEFAULT_SHELL.to_string() }
    };

    let config_path = get_config_path(args.config_path);

    let parsed_command_defs = file_handling::get_command_definitions(&config_path)?;

    let last_command_path = get_last_command_path(args.last_command_path);

    let last_command= file_handling::get_last_command(&last_command_path)?;

    let should_rerun_last_command;

    if args.rerun_last_command {
        if last_command.is_none() {
            // todo: This should be a warning printout"
            println!("Rerun last command was specified, but there is no previous command!");
            should_rerun_last_command = false;
        } else {
            should_rerun_last_command = true;
        }
    } else {
        should_rerun_last_command = false;
    }

    let selected_option = match should_rerun_last_command {
        true => Rerun(last_command.unwrap()),
        false => {
            for (index, command_def) in parsed_command_defs.iter().enumerate() {
                println!("{}", format_command_def(&index, &command_def));
            }

            if last_command.is_some() {
                println!("{}", format_command_def(&LAST_COMMAND_OPTION, &last_command.as_ref().unwrap().command));
            }
            command_selection::read_option_input(parsed_command_defs.len(), last_command)
        }
    };

    let args_as_string: String;
    let working_directory: Option<String>;

    let last_command_to_write: Option<LastCommandParameters>;

    match selected_option {
        Index(selected_index) => {
            let selected_command = &parsed_command_defs[selected_index];
            let defaults = interpolation::build_default_lookup(&selected_command.parameters);
            let interpolated_arguments = interpolation::interpolate_arguments(&defaults, &selected_command.command);

            args_as_string = interpolated_arguments.join(" ");
            working_directory = selected_command.working_directory.clone();
            last_command_to_write = Some(
                LastCommandParameters{
                    command: args_as_string.clone(),
                    working_directory: working_directory.clone()
                }
            )
        }
        Rerun(last_command) => {
            args_as_string = last_command.command;
            working_directory = last_command.working_directory.clone();
            // since we already loaded this, we don't need to write it again
            last_command_to_write = None;
        }
        Quit => {
            return Ok(());
        }
    }

    let mut command = Command::new(shell);
    if let Some(working_directory) = working_directory {
        let expanded_working_dir = shellexpand::tilde(working_directory.as_str());
        command.current_dir(expanded_working_dir.as_ref());
    }

    println!("Executing command:\n{}", args_as_string);
    if args.dry_run {
        println!("Dry run is specified, exiting without executing.");
        return Ok(());
    }

    if !args.force && !command_selection::confirm_command_should_run() {
        // Exit if command was not confirmed and was not forced
        return Ok(());
    }

    if args.skip_command_save {
      println!("Skipping command save was specified. Not (over)writing last command.");
    } else if let Some(last_command_to_write) = last_command_to_write {
        file_handling::write_last_command(&last_command_path, &last_command_to_write)?
    }

    // Give `-i` argument to start an interactive shell,
    // which will make it read ~/.rc or ~/.profile or whatever file
    command.args(vec!["-i", "-c", args_as_string.as_str()]);

    execution::execute_command(command)
}

fn main() -> ExitCode {
   match execute() {
       Ok(_) => ExitCode::SUCCESS,
       Err(e) => {
           eprintln!("{}", e);
           ExitCode::FAILURE
       }
   }
}
