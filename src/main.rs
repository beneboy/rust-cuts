use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt::Display;
use std::process::{Command, ExitCode};

use clap::Parser;
use itertools::Itertools;
use log::{debug, info, warn};

use command_selection::CommandSelectionResult::{Index, Quit, Rerun};

use crate::command_definitions::CommandExecutionTemplate;
use crate::command_selection::CommandRunResult;
use crate::error::Result;
use crate::interpolation::{get_template_context, get_templates, get_tokens, interpolate_command};

mod cli_args;
mod command_definitions;
mod command_selection;
mod file_handling;
mod interpolation;
mod execution;
mod error;


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

/// Get the defaults for a command, which may be the provided values from the last run or those
/// in the config.
/// Also return should_prompt if the values should be prompted for.
fn should_prompt_for_parameters(tokens: &HashSet<String>, provided_defaults: &Option<HashMap<String, String>>, is_rerun: bool) -> bool {
    if tokens.is_empty() {
        // If no tokens, then there should be no parameters and shouldn't be prompted
        return false;
    }

    return match provided_defaults.as_ref() {
        Some(provided_defaults) => {
            if is_rerun {
                // If any of the tokens don't exist in the provided defaults,
                // then we should prompt.
                tokens.iter().any(|token| {
                    !provided_defaults.contains_key(token)
                })
            } else {
                // If it's not a rerun, then we should prompt, even if there are defaults
                true
            }
        },
        None => {
            // Provided defaults is none, we should prompt
            true
        }
    }
}

fn execute() -> Result<()> {
    let args = cli_args::Args::parse();

    let shell = match env::var("SHELL") {
        Ok(shell) => { shell }
        Err(_) => { DEFAULT_SHELL.to_string() }
    };

    let config_path = get_config_path(args.config_path);
    debug!("Config path: `{}`", config_path);

    let parsed_command_defs = file_handling::get_command_definitions(&config_path)?;

    let last_command_path = get_last_command_path(args.last_command_path);

    let last_command= file_handling::get_last_command(&last_command_path)?;

    let should_rerun_last_command;

    if args.rerun_last_command {
        if last_command.is_none() {
            warn!("Rerun last command was specified, but there is no previous command!");
            should_rerun_last_command = false;
        } else {
            should_rerun_last_command = true;
        }
    } else {
        should_rerun_last_command = false;
    }

    let selected_option = match should_rerun_last_command {
        true => Rerun(last_command.clone().unwrap()),
        false => {
            for (index, command_def) in parsed_command_defs.iter().enumerate() {
                println!("{}", format_command_def(&index, &command_def));
            }

            if let Some(last_command) = last_command.clone() {
                let template_context = last_command.template_context;
                let interpolated_last_command = interpolate_command(&template_context, &get_templates(&last_command.command)?)?;
                println!("{}", format_command_def(&LAST_COMMAND_OPTION, &interpolated_last_command.join(" ")));
            }

            command_selection::read_option_input(parsed_command_defs.len(), &last_command)
        }
    };

    let mut execution_context: CommandExecutionTemplate;
    let defaults: Option<HashMap<String, String>>;

    match selected_option {
        Index(selected_index) => {
            let selected_command = &parsed_command_defs[selected_index];
            defaults = interpolation::build_default_lookup(&selected_command.parameters);
            execution_context = CommandExecutionTemplate::from_command_definition(selected_command, None);
        }
        Rerun(last_command) => {
            execution_context = last_command.clone();
            defaults = last_command.template_context.clone();
        }
        Quit => {
            return Ok(());
        }
    }

    let templates  = get_templates(&execution_context.command)?;

    let tokens = get_tokens(&templates)?;

    let mut args_as_string: String;

    let mut prompt_for_parameters = should_prompt_for_parameters(&tokens, &defaults, last_command.is_some());

    let mut template_context = None;

    loop {
        if tokens.is_empty() {
            template_context = None;
        } else if prompt_for_parameters {
            // On first loop, the defaults should be the normal defaults
            // Once template_context is set, that should be used as the default
            template_context = get_template_context(&tokens,
                                                    if template_context.is_none() {
                                                        &defaults
                                                    } else  {
                                                        &template_context
                                                    }
            )
        } else {
            template_context = defaults.clone();
        };

        args_as_string = interpolate_command(&template_context, &templates)?.join(" ");

        println!("Executing command:\n{}", args_as_string);

        if let Some(environment) = execution_context.environment.as_ref() {
            println!("With environment:");
            for (key, value) in environment.iter().sorted() {
                println!("\t\"{}\": \"{}\"", key, value);
            }
        }
        if args.dry_run {
            println!("Dry run is specified, exiting without executing.");
            return Ok(());
        }
        if args.force {
            // Force run - break loop
            break;
        }

        match command_selection::confirm_command_should_run(!tokens.is_empty()) {
            CommandRunResult::Yes => {
                // Break loop, do run
                execution_context.template_context = template_context.clone();
                break;
            }
            CommandRunResult::No => {
                // Exit if command was not confirmed and was not forced
                return Ok(());
            }
            CommandRunResult::ChangeParams => {
                // Continue the loop, params are re-requested if missing_defaults becomes true
                prompt_for_parameters = true;
            }
        }
    }

    let mut command = Command::new(shell);
    if let Some(working_directory) = &execution_context.working_directory {
        let expanded_working_dir = shellexpand::tilde(working_directory.as_str());
        command.current_dir(expanded_working_dir.as_ref());
    }

    if args.skip_command_save {
        info!("Skipping command save was specified. Not (over)writing last command.");
    } else {
        file_handling::write_last_command(&last_command_path, &execution_context)?
    }

    // Give `-i` argument to start an interactive shell,
    // which will make it read ~/.rc or ~/.profile or whatever file
    command.args(vec!["-i", "-c", args_as_string.as_str()]);

    execution::execute_command(command, execution_context.environment)
}

fn main() -> ExitCode {
    env_logger::init();

    match execute() {
       Ok(_) => ExitCode::SUCCESS,
       Err(e) => {
           eprintln!("{}", e);
           ExitCode::FAILURE
       }
   }
}
