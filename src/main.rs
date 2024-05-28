use std::collections::{HashMap, HashSet};
use std::env;
use std::process::{Command, ExitCode};

use clap::Parser;
use itertools::Itertools;
use log::{debug, info, warn};

use crate::cli_args::Args;
use command_selection::CommandChoice::{Index, Quit, Rerun};

use crate::command_definitions::CommandExecutionTemplate;
use crate::command_selection::RunChoice;
use crate::error::{Error, Result};
use crate::interpolation::{get_template_context, get_templates, get_tokens, interpolate_command};

mod cli_args;
mod command_definitions;
mod command_selection;
mod error;
mod execution;
mod file_handling;
mod interpolation;

static DEFAULT_CONFIG_PATH: &str = "~/.rust-cuts/commands.yml";
static DEFAULT_LAST_COMMAND_PATH: &str = "~/.rust-cuts/last_command.yml";
static LAST_COMMAND_OPTION: &str = "r";

static DEFAULT_SHELL: &str = "/bin/bash";

fn get_config_path(config_path_arg: &Option<String>) -> String {
    let config_path = match config_path_arg {
        Some(last_command_path) => last_command_path,
        None => DEFAULT_CONFIG_PATH,
    };

    shellexpand::tilde(config_path).to_string()
}

fn get_last_command_path(last_command_path_arg: &Option<String>) -> String {
    let last_command_path = match last_command_path_arg {
        Some(last_command_path) => last_command_path,
        None => DEFAULT_LAST_COMMAND_PATH,
    };

    shellexpand::tilde(last_command_path).to_string()
}

/// Parameters should not be prompted for if:
/// 1. There are no tokens to interpolate!
/// 2. A command is being re-run, and all parameters were provided previously.*
/// *: A re-run is based on the previous definition of the command, therefore the only way the
/// command would not have all the parameters is if the last command YAML file was edited and had
/// some parameters removed.
fn get_should_prompt_for_parameters(
    tokens: &HashSet<String>,
    provided_defaults: &Option<HashMap<String, String>>,
    is_rerun: bool,
) -> bool {
    if tokens.is_empty() {
        // If no tokens, then there should be no parameters and shouldn't be prompted
        return false;
    }

    if !is_rerun {
        return true;
    }

    return match provided_defaults.as_ref() {
        Some(provided_defaults) => {
            // If any of the tokens don't exist in the provided defaults,
            // then we should prompt.
            tokens
                .iter()
                .any(|token| !provided_defaults.contains_key(token))
        }
        None => {
            // Provided defaults is none, we should prompt
            true
        }
    };
}

fn get_should_rerun_last_command(
    args: &Args,
    last_command: &Option<CommandExecutionTemplate>,
) -> Result<bool> {
    if !args.rerun_last_command {
        return Ok(false);
    }

    if args.command_index.is_some() {
        // Can't rerun if an index is specified, doesn't make sense
        return Err(Error::RerunWithIndex);
    }

    if last_command.is_none() {
        warn!("Rerun last command was specified, but there is no previous command!");
        return Ok(false);
    }

    Ok(true)
}

fn execute() -> Result<()> {
    let args = cli_args::Args::parse();

    let shell = match env::var("SHELL") {
        Ok(shell) => shell,
        Err(_) => DEFAULT_SHELL.to_string(),
    };

    let config_path = get_config_path(&args.config_path);
    debug!("Config path: `{}`", config_path);

    let parsed_command_defs = file_handling::get_command_definitions(&config_path)?;

    let last_command_path = get_last_command_path(&args.last_command_path);

    let last_command = file_handling::get_last_command(&last_command_path)?;

    let should_rerun_last_command = get_should_rerun_last_command(&args, &last_command)?;

    let selected_option;

    if should_rerun_last_command {
        if args.command_index.is_some() {
            // Can't rerun if an index is specified, doesn't make sense
            return Err(Error::RerunWithIndex);
        }
        selected_option = Rerun(last_command.clone().unwrap());
    } else if let Some(index) = args.command_index {
        if index >= parsed_command_defs.len() {
            return Err(Error::Misc(
                format!("Command index out of range: {index}!").to_string(),
            ));
        }

        selected_option = Index(index);
    } else {
        selected_option =
            command_selection::prompt_for_command_choice(&parsed_command_defs, &last_command)?;
    }

    let mut execution_context: CommandExecutionTemplate;
    let defaults: Option<HashMap<String, String>>;

    match selected_option {
        Index(selected_index) => {
            let selected_command = &parsed_command_defs[selected_index];
            defaults = interpolation::build_default_lookup(&selected_command.parameters);
            execution_context = CommandExecutionTemplate::from_command_definition(selected_command);
        }
        Rerun(last_command) => {
            execution_context = last_command.clone();
            defaults = last_command.template_context.clone();
        }
        Quit => {
            return Ok(());
        }
    }

    let templates = get_templates(&execution_context.command)?;

    let tokens = get_tokens(&templates);

    let mut args_as_string: String;

    let mut should_prompt_for_parameters =
        get_should_prompt_for_parameters(&tokens, &defaults, last_command.is_some());

    let mut template_context = None;

    loop {
        if tokens.is_empty() {
            template_context = None;
        } else if should_prompt_for_parameters {
            // On first loop, the defaults should be the normal defaults
            // Once template_context is set, that should be used as the default
            template_context = get_template_context(
                &tokens,
                if template_context.is_none() {
                    &defaults
                } else {
                    &template_context
                },
            )?;
        } else {
            template_context.clone_from(&defaults);
        };

        args_as_string = interpolate_command(&template_context, &templates)?.join(" ");

        print_command_and_environment(&execution_context, &args_as_string);
        if args.dry_run {
            println!("Dry run is specified, exiting without executing.");
            return Ok(());
        }
        if args.force {
            // Force run - break loop
            break;
        }

        match command_selection::confirm_command_should_run(!tokens.is_empty())? {
            RunChoice::Yes => {
                // Break loop, do run
                execution_context
                    .template_context
                    .clone_from(&template_context);
                break;
            }
            RunChoice::No => {
                // Exit if command was not confirmed and was not forced
                return Ok(());
            }
            RunChoice::ChangeParams => {
                // Continue the loop, params are re-requested if missing_defaults becomes true
                should_prompt_for_parameters = true;
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
        file_handling::write_last_command(&last_command_path, &execution_context)?;
    }

    // Give `-i` argument to start an interactive shell,
    // which will make it read ~/.rc or ~/.profile or whatever file
    command.args(vec!["-i", "-c", args_as_string.as_str()]);

    execution::execute_command(command, execution_context.environment)
}

fn print_command_and_environment(
    execution_context: &CommandExecutionTemplate,
    args_as_string: &String,
) {
    println!("Executing command:\n{args_as_string}");

    if let Some(environment) = execution_context.environment.as_ref() {
        println!("With environment:");
        for (key, value) in environment.iter().sorted() {
            println!("\t\"{key}\": \"{value}\"");
        }
    }
}

fn main() -> ExitCode {
    env_logger::init();

    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
