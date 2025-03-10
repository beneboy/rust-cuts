use std::collections::HashMap;
use std::io::{stdin, stdout, Write};
use indexmap::IndexSet;
use itertools::Itertools;
use rust_cuts_core::command_definitions::ParameterDefinition;
use rust_cuts_core::error::Result;

/// Prompts the user for a parameter value
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

/// Confirms with the user whether the command should be run
pub fn confirm_command_should_run(has_params: bool) -> Result<super::types::RunChoice> {
    use super::types::RunChoice;

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

/// Fills in parameter values by prompting the user
pub fn fill_parameter_values(
    tokens: &IndexSet<String>,
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

/// Helper function to create or update a parameter definition
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