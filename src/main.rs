use std::{env, io};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Write};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use clap::Parser;
use leon::Template;
use serde::{Deserialize, Serialize};

use crate::CommandSelectionResult::{Index, Quit, Rerun};

mod cli_args;

#[derive(Deserialize, Debug)]
struct ParameterDefinition {
    name: String,
    default: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CommandDefinition {
    command: Vec<String>,
    name: Option<String>,
    working_directory: Option<String>,
    parameters: Option<Vec<ParameterDefinition>>,
}

#[derive(Deserialize, Serialize, Debug)]
struct LastCommandParameters {
    command: String,
    working_directory: Option<String>
}

impl CommandDefinition {
    fn to_string(&self) -> String {
        match &self.name {
            Some(name) => name.clone(),
            None => {
                self.command.join(" ")
            }
        }
    }
}

impl Display for CommandDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())?;
        Ok(())
    }
}

enum CommandSelectionResult {
    Index(usize),
    Rerun,
    Quit,
}

fn build_default_lookup(definitions: &Option<Vec<ParameterDefinition>>) -> Option<HashMap<String, String>> {
    if let Some(definitions) = definitions.as_ref() {
        let mut defaults: HashMap<String, String> = HashMap::new();
        for definition in definitions {
            if let Some(default) = &definition.default {
                defaults.insert(definition.name.clone(), default.clone());
            }
        }

        Some(defaults)
    } else {
        None
    }
}

fn read_option_input(max: usize, include_rerun: bool) -> CommandSelectionResult {
    loop {
        let rerun_text = match include_rerun {
            true => ", or `l` to re-run last",
            false => ""
        };
        // Prompt the user for input
        print!("Enter an option (0-{}{}. Quit with `q`): ", max - 1, rerun_text);
        io::stdout().flush().expect("Failed to flush stdout");

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        input = input.trim().to_lowercase();

        if input == "q" {
            return Quit;
        }

        if include_rerun && input == "r" {
            return Rerun;
        }

        // Parse input as usize
        match input.parse::<usize>() {
            Ok(index) => {
                if index < max {
                    return Index(index);
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

fn prompt_value(variable_name: &str, default_value: Option<String>) -> String {
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

        if read_value == "" {
            if default_value.is_some() {
                return default_value.unwrap();
            }
        } else {
            return read_value;
        }
    }
}

fn confirm_command_should_run() -> bool {
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

fn interpolate_arguments(defaults: &Option<HashMap<String, String>>, arguments: &Vec<String>) -> Vec<String> {
    let mut interpolated_arguments: Vec<String> = vec![];
    for argument in arguments.iter() {
        match Template::parse(argument.as_ref()) {
            Ok(template) => {
                let mut context: HashMap<String, String> = HashMap::new();
                for key in template.keys() {
                    let default_value = match defaults {
                        Some(defaults) => { defaults.get(&key.to_string()) }
                        None => { None }
                    };

                    let value = prompt_value(key, default_value.cloned());

                    context.insert(key.parse().unwrap(), value);
                }

                if context.is_empty() {
                    interpolated_arguments.push(argument.clone())
                } else {
                    match template.render(&context) {
                        Ok(rendered_argument) => {
                            interpolated_arguments.push(rendered_argument)
                        }
                        Err(err) => {
                            panic!("{}", err)
                        }
                    }
                }
            }
            Err(err) => {
                panic!("{}", err.to_string())
            }
        }
    }
    return interpolated_arguments;
}

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

fn get_reader(file_description: &str, path: &str) -> Result<File, String> {
    return match File::open(path) {
        Ok(reader) => Ok(reader),
        Err(e) => Err(format!("Could not read {} file at `{}`: {}", file_description, path, e))
    };
}

fn get_last_command_reader(last_command_path: &String) -> Result<Option<File>, String> {
    if !Path::exists(Path::new(last_command_path)) {
        return Ok(None);
    }

    return match get_reader("last command", last_command_path) {
        Ok(f) => Ok(Some(f)),
        Err(e) => Err(e),
    };
}

fn get_last_command(last_command_path: &String) -> Result<Option<LastCommandParameters>, String> {
    let last_command_reader = get_last_command_reader(last_command_path)?;
    let Some(last_command_reader) = last_command_reader else {
        return Ok(None);
    };

    let last_command_parameter: serde_yaml::Result<LastCommandParameters> = serde_yaml::from_reader(last_command_reader);
    
    match last_command_parameter {
        Ok(last_command_parameter) => {
            Ok(Some(last_command_parameter))
        }
        Err(e) => {
            Err(format!("Error reading last command at at `{}`: {}", last_command_path, e).to_string())
        }
    }
}

fn execute_command(mut command: Command) -> ExitCode {
    let spawn_result = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    let Ok(mut child) = spawn_result else {
        eprintln!("Error spawning subprocess: {}", spawn_result.unwrap_err());
        return ExitCode::FAILURE;
    };

    let child_process_result = child.wait();
    return match child_process_result {
        Ok(exit_status) => {
            if exit_status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("Subprocess didn't give a result: {}", e);
            return ExitCode::FAILURE;
        }
    };
}

fn write_last_command(path: &str, last_command: &LastCommandParameters) -> Result<(), String> {
    let f = File::create(path);

    let Ok( f) = f else {
        return Err(format!("Error opening last command path at `{}` for writing: {}", path, f.unwrap_err()));
    };

    return match serde_yaml::to_writer(f, &last_command) {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(format!("Error writing to last command path at `{}`: {}", path, e))
        }
    };
}


fn main() -> ExitCode {
    let args = cli_args::Args::parse();

    let shell = match env::var("SHELL") {
        Ok(shell) => { shell }
        Err(_) => { DEFAULT_SHELL.to_string() }
    };

    let config_path = get_config_path(args.config_path);

    let parsed_command_defs = match read_command_definitions(&config_path) {
        Ok(value) => value,
        Err(value) => {
            eprintln!("{}", value);
            return ExitCode::FAILURE;
        },
    };

    let last_command_path = get_last_command_path(args.last_command_path);

    let last_command= get_last_command(&last_command_path);

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
        true => Rerun,
        false => {
            for (index, command_def) in parsed_command_defs.iter().enumerate() {
                println!("[{index}]: {}", command_def)
            }

            if last_command.is_some() {
                println!("[{}] {}", LAST_COMMAND_OPTION, last_command.as_ref().unwrap().command);
            }
            read_option_input(parsed_command_defs.len(), last_command.is_some())
        }
    };

    let args_as_string: String;
    let working_directory: Option<String>;

    let last_command_to_write: Option<LastCommandParameters>;

    match selected_option {
        Index(selected_index) => {
            let selected_command = &parsed_command_defs[selected_index];
            let defaults = build_default_lookup(&selected_command.parameters);
            let interpolated_arguments = interpolate_arguments(&defaults, &selected_command.command);

            args_as_string = interpolated_arguments.join(" ");
            working_directory = selected_command.working_directory.clone();
            last_command_to_write = Some(
                LastCommandParameters{
                    command: args_as_string.clone(),
                    working_directory: working_directory.clone()
                }
            )
        }
        Rerun => {
            match last_command {
                Some(last_command) => {
                    args_as_string = last_command.command;
                    working_directory = last_command.working_directory.clone();
                    // since we already loaded this, we don't need to write it again
                    last_command_to_write = None;
                }
                None => {
                    // This feels dangerous, although this combination should be impossible
                    eprintln!("Rerun option specified but no last command exists!");
                    return ExitCode::FAILURE;
                }
            }
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

    if !args.force && !confirm_command_should_run() {
        // Exit if command was not confirmed and was not forced
        return ExitCode::SUCCESS;
    }

    if args.skip_command_save {
      println!("Skipping command save was specified. Not (over)writing last command.");
    } else {
        match last_command_to_write {
            Some(last_command_to_write) => {
                let write_result = write_last_command(&last_command_path, &last_command_to_write);

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

    return execute_command(command);
}

fn read_command_definitions(config_path: &String) -> Result<Vec<CommandDefinition>, String> {
    let config_reader_result = get_reader("config", &config_path);

    let parsing_result: serde_yaml::Result<Vec<CommandDefinition>>;

    {
        let Ok(ref config_reader) = config_reader_result else {
            return Err(format!("{}", config_reader_result.unwrap_err()))
        };

        parsing_result = serde_yaml::from_reader(config_reader);
    }

    let Ok(parsed_command_defs) = parsing_result else {
        return Err(format!(
            "Failed to parse config file at `{}`:\n{}",
            config_path, parsing_result.unwrap_err()
        ));
    };

    if parsed_command_defs.len() == 0 {
        return Err(format!("No commands were found in the command definition YAML. Is `{}` empty?", config_path));
    }
    Ok(parsed_command_defs)
}
