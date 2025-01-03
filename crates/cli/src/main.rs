use std::collections::{HashMap, HashSet};
use std::env;
use std::io::{stdout, Write};
use std::process::{Command, ExitCode};

use clap::Parser;
use crossterm::{cursor, queue, terminal};
use crossterm::terminal::{Clear, ClearType, disable_raw_mode};
use itertools::Itertools;
use log::{debug, info, warn};

use command_selection::CommandChoice::{Index, Quit, Rerun};
use command_selection::get_template_context;
use rust_cuts_core::{config, file_handling, interpolation};
use rust_cuts_core::command_definitions::{CommandDefinition, CommandExecutionTemplate};
use rust_cuts_core::config::DEFAULT_SHELL;
use rust_cuts_core::error::{Error, Result};
use rust_cuts_core::execution;

use crate::cli_args::Args;
use crate::command_selection::{CommandChoice, RunChoice};
use rust_cuts_core::interpolation::{get_templates, get_tokens, interpolate_command};

pub mod command_selection;
mod cli_args;

const LAST_COMMAND_OPTION: char = 'r';

/// Parameters should not be prompted for if:
/// 1. There are no tokens to interpolate!
/// 2. A command is being re-run, and all parameters were provided previously.*
///
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

    match provided_defaults.as_ref() {
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
    }
}

fn get_rerun_request_is_valid(args: &Args) -> Result<bool> {
    if !args.rerun_last_command {
        return Ok(false);
    }

    if args.command_index.is_some() {
        // Can't rerun if an index is specified, doesn't make sense
        return Err(Error::RerunWithIndex);
    }

    Ok(true)
}

fn execute() -> Result<()> {
    let args = cli_args::Args::parse();

    let shell = env::var("SHELL").unwrap_or_else(|_| DEFAULT_SHELL.to_string());

    let config_path = config::get_config_path(&args.config_path);
    debug!("Config path: `{}`", config_path);

    let parsed_command_defs = file_handling::get_command_definitions(&config_path)?;

    let last_command_path = config::get_last_command_path(&args.last_command_path);

    let last_command = file_handling::get_last_command(&last_command_path)?;

    let rerun_option = if get_rerun_request_is_valid(&args)? {
        if let Some(last_command) = &last_command {
            Some(Rerun(last_command.clone()))
        } else {
            warn!("Rerun last command was specified, but there is no previous command!");
            None
        }
    } else {
        None
    };

    let selected_option = match rerun_option {
        None => get_selected_option(&args, &parsed_command_defs, last_command.as_ref())?,
        Some(rerun_option) => rerun_option,
    };

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
            let mut stdout = stdout();
            queue!(stdout, Clear(ClearType::All),)?;
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
    if let Some(working_directory) = config::expand_working_directory(&execution_context.working_directory) {
        command.current_dir(working_directory);
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

fn get_selected_option(
    args: &Args,
    parsed_command_defs: &[CommandDefinition],
    last_command: Option<&CommandExecutionTemplate>,
) -> Result<CommandChoice> {
    if let Some(index) = args.command_index {
        if index >= parsed_command_defs.len() {
            return Err(Error::Misc(format!("Command index out of range: {index}!")));
        }

        Ok(Index(index))
    } else {
        let selected_option =
            command_selection::prompt_for_command_choice(parsed_command_defs, last_command)?;

        let mut stdout = stdout();

        let (_, height) = terminal::size()?; // todo: Do this before so we only scroll to end of commands not to bottom of terminal

        queue!(
            stdout,
            cursor::MoveToColumn(0),
            cursor::MoveToRow(height),
            terminal::Clear(ClearType::CurrentLine)
        )?;
        disable_raw_mode()?;
        stdout.flush()?;
        Ok(selected_option)
    }
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
