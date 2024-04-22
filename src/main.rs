use std::env;
use std::process::{Command, ExitCode};

use clap::Parser;

use command_definitions::LastCommandParameters;
use command_selection::CommandSelectionResult::{Index, Quit, Rerun};

mod cli_args;
mod command_definitions;
mod command_selection;
mod file_handling;
mod interpolation;
mod execution;

static DEFAULT_CONFIG_PATH: &str = "~/.rust-cuts/commands.yml";
static DEFAULT_LAST_COMMAND_PATH: &str = "~/.rust-cuts/last_command.yml";
static LAST_COMMAND_OPTION: &str = "r";

static DEFAULT_SHELL: &str = "/bin/bash";

fn get_config_path(config_path_arg: Option<String>) -> String {
    let ref config_path = config_path_arg.unwrap_or(DEFAULT_CONFIG_PATH.to_string());
    shellexpand::tilde(config_path).to_string()
}

fn get_last_command_path(last_command_path_arg: Option<String>) -> String {
    let ref last_command_path = last_command_path_arg.unwrap_or(DEFAULT_LAST_COMMAND_PATH.to_string());
    shellexpand::tilde(last_command_path).to_string()
}


fn main() -> ExitCode {
    let args = cli_args::Args::parse();

    let shell = match env::var("SHELL") {
        Ok(shell) => { shell }
        Err(_) => { DEFAULT_SHELL.to_string() }
    };

    let config_path = get_config_path(args.config_path);

    let parsed_command_defs = match file_handling::get_command_definitions(&config_path) {
        Ok(value) => value,
        Err(value) => {
            eprintln!("{}", value);
            return ExitCode::FAILURE;
        },
    };

    let last_command_path = get_last_command_path(args.last_command_path);

    let last_command= file_handling::get_last_command(&last_command_path);

    let Ok(last_command) = last_command else {
        eprintln!("{}", last_command.unwrap_err());
        return ExitCode::FAILURE;
    };

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
                println!("[{index}]: {}", command_def)
            }

            if last_command.is_some() {
                println!("[{}] {}", LAST_COMMAND_OPTION, last_command.as_ref().unwrap().command);
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
            return ExitCode::SUCCESS;
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
        return ExitCode::SUCCESS;
    }

    if !args.force && !command_selection::confirm_command_should_run() {
        // Exit if command was not confirmed and was not forced
        return ExitCode::SUCCESS;
    }

    if args.skip_command_save {
      println!("Skipping command save was specified. Not (over)writing last command.");
    } else {
        match last_command_to_write {
            Some(last_command_to_write) => {
                let write_result = file_handling::write_last_command(&last_command_path, &last_command_to_write);

                if write_result.is_err() {
                    eprintln!("{}", write_result.unwrap_err());
                    return ExitCode::FAILURE;
                }
            }
            None => {}
        }
    }

    // Give `-i` argument to start an interactive shell,
    // which will make it read ~/.rc or ~/.profile or whatever file
    command.args(vec!["-i", "-c", args_as_string.as_str()]);

    return execution::execute_command(command);
}
