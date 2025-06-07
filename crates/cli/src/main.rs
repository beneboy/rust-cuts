use clap::Parser;
use command_selection::fill_parameter_values;
use command_selection::CommandChoice::{Index, Quit, Rerun};
use crossterm::terminal::{disable_raw_mode, Clear, ClearType};
use crossterm::{cursor, queue, terminal};
use itertools::Itertools;
use log::{debug, info, warn};
use rust_cuts_core::command_definitions::{
    CommandDefinition, CommandExecutionTemplate, ParameterDefinition, TemplateParser,
};
use rust_cuts_core::config::DEFAULT_SHELL;
use rust_cuts_core::error::Error::CommandNotFound;
use rust_cuts_core::error::{Error, Result};
use rust_cuts_core::execution;
use rust_cuts_core::{config, file_handling, interpolation};
use std::collections::HashMap;
use std::env;
use std::io::{stdout, Write};
use std::process::{Command, ExitCode};

use crate::arguments::validation::should_prompt_for_parameters;
use crate::arguments::{process_command_line, Provider};
use crate::cli_args::Args;
use crate::command_selection::CommandChoice::CommandId;
use crate::command_selection::{CommandChoice, RunChoice};
use rust_cuts_core::interpolation::interpolate_command;

/// Type alias for the complex return type of `setup_execution_context`
type ExecutionContextResult = (
    CommandExecutionTemplate,
    Option<HashMap<String, ParameterDefinition>>,
    bool,
);

mod arguments;
mod cli_args;
pub mod command_selection;

fn get_rerun_request_is_valid(args: &Args) -> Result<bool> {
    if !args.rerun_last_command {
        return Ok(false);
    }

    if args.command_id_or_index.is_some() {
        // Can't rerun if an index is specified, doesn't make sense
        return Err(Error::RerunWithIndex);
    }

    Ok(true)
}

/// Initialize configuration and load command definitions
fn initialize_config(
    args: &Args,
) -> Result<(
    Vec<CommandDefinition>,
    String,
    Option<CommandExecutionTemplate>,
)> {
    let config_path = config::get_config_path(&args.config_path);
    debug!("Config path: `{}`", config_path);

    let parsed_command_defs = file_handling::get_command_definitions(&config_path)?;
    let last_command_path = config::get_last_command_path(&args.last_command_path);
    let last_command = file_handling::get_last_command(&last_command_path)?;

    Ok((parsed_command_defs, last_command_path, last_command))
}

/// Determine which command to execute based on arguments and user selection
fn determine_command_choice(
    args: &Args,
    parsed_command_defs: &[CommandDefinition],
    last_command: Option<&CommandExecutionTemplate>,
) -> Result<CommandChoice> {
    let rerun_option = if get_rerun_request_is_valid(args)? {
        if let Some(last_command) = last_command {
            Some(Rerun(last_command.clone()))
        } else {
            warn!("Rerun last command was specified, but there is no previous command!");
            None
        }
    } else {
        None
    };

    match rerun_option {
        None => get_selected_option(args, parsed_command_defs, last_command),
        Some(rerun_option) => Ok(rerun_option),
    }
}

/// Setup execution context and parameter definitions from the selected command
fn setup_execution_context(
    selected_option: CommandChoice,
    parsed_command_defs: &[CommandDefinition],
) -> Result<ExecutionContextResult> {
    let execution_context: CommandExecutionTemplate;
    let parameter_definitions: Option<HashMap<String, ParameterDefinition>>;
    let mut is_rerun = false;

    match selected_option {
        Index(selected_index) => {
            let selected_command = &parsed_command_defs[selected_index];
            parameter_definitions =
                interpolation::build_parameter_lookup(&selected_command.parameters);
            execution_context = CommandExecutionTemplate::from_command_definition(selected_command);
        }
        CommandId(command_id) => {
            let selected_command = parsed_command_defs
                .iter()
                .find(|cmd| {
                    if let Some(id) = &cmd.id {
                        return *id == command_id;
                    }
                    false
                })
                .ok_or(CommandNotFound(command_id))?;
            parameter_definitions =
                interpolation::build_parameter_lookup(&selected_command.parameters);
            execution_context = CommandExecutionTemplate::from_command_definition(selected_command);
        }
        Rerun(last_command) => {
            execution_context = last_command.clone();
            parameter_definitions = last_command.template_context.clone();
            is_rerun = true;
        }
        Quit => {
            let mut stdout = stdout();
            queue!(stdout, Clear(ClearType::All),)?;
            // This is handled in the caller
            unreachable!("Quit should be handled before calling this function");
        }
    }

    Ok((execution_context, parameter_definitions, is_rerun))
}

/// Handle parameter processing and user interaction loop
fn process_parameters_and_confirm(
    args: &Args,
    mut execution_context: CommandExecutionTemplate,
    parameter_definitions: Option<&HashMap<String, ParameterDefinition>>,
    is_rerun: bool,
) -> Result<(CommandExecutionTemplate, String)> {
    let templates = execution_context.get_templates()?;
    let tokens = execution_context.get_ordered_context_variables()?;

    // Process command-line parameters first
    let mut filled_parameters = if tokens.is_empty() {
        None
    } else {
        process_command_line(args.get_style()?, &execution_context, parameter_definitions)?
    };

    // Initial prompt check
    let mut need_to_prompt = should_prompt_for_parameters(
        &tokens,
        filled_parameters.as_ref(),
        is_rerun,
        &args.get_style()?,
    );

    let args_as_string = loop {
        // Handle parameters based on our current state
        if tokens.is_empty() {
            // No tokens, no parameters needed
            filled_parameters = None;
        } else if need_to_prompt {
            // Prompt the user for parameter values
            filled_parameters =
                fill_parameter_values(&tokens, parameter_definitions, filled_parameters.as_ref())?;
            // After prompting, we don't need to prompt again unless the user chooses to change params
        }
        // Don't overwrite parameters if we already have them and don't need to prompt

        // Build template context from parameters
        let template_context: HashMap<String, String> = match &filled_parameters {
            Some(param_defs) => param_defs
                .iter()
                .filter_map(|(name, param_def)| {
                    param_def
                        .default
                        .as_ref()
                        .map(|default| (name.clone(), default.clone()))
                })
                .collect(),
            None => HashMap::new(),
        };

        let interpolated_command = interpolate_command(&template_context, &templates)?.join(" ");

        print_command_and_environment(&execution_context, &interpolated_command);

        if args.dry_run {
            println!("Dry run is specified, exiting without executing.");
            return Err(Error::Misc("Dry run completed".to_string()));
        }

        if args.force {
            // Force run - break loop
            execution_context.template_context = filled_parameters;
            break interpolated_command;
        }

        match command_selection::confirm_command_should_run(!tokens.is_empty())? {
            RunChoice::Yes => {
                // Break loop, do run
                execution_context.template_context = filled_parameters;
                break interpolated_command;
            }
            RunChoice::No => {
                // Exit if command was not confirmed and was not forced
                return Err(Error::Misc(
                    "Command execution cancelled by user".to_string(),
                ));
            }
            RunChoice::ChangeParams => {
                // Continue the loop, params are re-requested if missing_defaults becomes true
                need_to_prompt = true;
            }
        }
    };

    Ok((execution_context, args_as_string))
}

fn execute() -> Result<()> {
    let args = cli_args::Args::parse();
    let shell = env::var("SHELL").unwrap_or_else(|_| DEFAULT_SHELL.to_string());

    // Initialize configuration and load commands
    let (parsed_command_defs, last_command_path, last_command) = initialize_config(&args)?;

    // Determine which command to execute
    let selected_option =
        determine_command_choice(&args, &parsed_command_defs, last_command.as_ref())?;

    // Handle quit option early
    if matches!(selected_option, Quit) {
        let mut stdout = stdout();
        queue!(stdout, Clear(ClearType::All),)?;
        return Ok(());
    }

    // Setup execution context
    let (execution_context, parameter_definitions, is_rerun) =
        setup_execution_context(selected_option, &parsed_command_defs)?;

    // Process parameters and get user confirmation
    let (execution_context, args_as_string) = match process_parameters_and_confirm(
        &args,
        execution_context,
        parameter_definitions.as_ref(),
        is_rerun,
    ) {
        Ok(result) => result,
        Err(Error::Misc(msg)) if msg.contains("Dry run") || msg.contains("cancelled") => {
            // These are expected "errors" that should exit cleanly
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // Save command if not skipping
    if args.skip_command_save {
        info!("Skipping command save was specified. Not (over)writing last command.");
    } else {
        file_handling::write_last_command(&last_command_path, &execution_context)?;
    }

    // Execute the command
    let mut command = Command::new(shell);
    if let Some(working_directory) =
        config::expand_working_directory(&execution_context.working_directory)
    {
        command.current_dir(working_directory);
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
    if let Some(command_id_or_index) = &args.command_id_or_index {
        if let Ok(index) = command_id_or_index.parse::<usize>() {
            if index >= parsed_command_defs.len() {
                return Err(Error::Misc(format!("Command index out of range: {index}!")));
            }

            Ok(Index(index))
        } else {
            Ok(CommandId(command_id_or_index.clone()))
        }
    } else {
        let selected_option =
            command_selection::prompt_for_command_choice(parsed_command_defs, last_command)?;

        let mut stdout = stdout();

        let (_, height) = terminal::size()?; // todo: Do this before so we only scroll to end of commands not to bottom of terminal

        queue!(
            stdout,
            cursor::MoveToColumn(0),
            cursor::MoveToRow(height),
            Clear(ClearType::CurrentLine)
        )?;
        disable_raw_mode()?;
        stdout.flush()?;
        Ok(selected_option)
    }
}

fn print_command_and_environment(
    execution_context: &CommandExecutionTemplate,
    args_as_string: &str,
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
