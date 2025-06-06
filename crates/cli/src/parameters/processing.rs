use crate::parameters::mode::ParameterMode;
use indexmap::IndexSet;
use rust_cuts_core::command_definitions::{
    CommandExecutionTemplate, ParameterDefinition, TemplateParser,
};
use rust_cuts_core::error::Error::{MissingParameter, ParameterCountMismatch, ParameterFormat};
use rust_cuts_core::error::Result;
use std::collections::HashMap;

/// Process command-line parameters based on the parameter mode
///
/// Takes the parameter mode, execution context, and existing parameter definitions,
/// and returns a map of parameter definitions with values filled from command-line arguments.
pub fn process_command_line_parameters(
    parameter_mode: ParameterMode,
    execution_template: &CommandExecutionTemplate,
    parameter_definitions: &Option<HashMap<String, ParameterDefinition>>,
) -> Result<Option<HashMap<String, ParameterDefinition>>> {
    let ordered_tokens = execution_template.get_ordered_context_variables()?;

    if ordered_tokens.is_empty() {
        return Ok(None);
    }

    // Base parameter definitions to start with
    let mut param_defs = match parameter_definitions {
        Some(defs) => defs.clone(),
        None => HashMap::new(),
    };

    match parameter_mode {
        ParameterMode::None => {
            // No parameters provided, return defaults or None
            return if param_defs.is_empty() {
                Ok(None)
            } else {
                Ok(Some(param_defs))
            };
        }

        ParameterMode::Named(named_params) => {
            process_named_parameters(&named_params, &ordered_tokens, &mut param_defs)?;
        }

        ParameterMode::Positional(positional_params) => {
            process_positional_parameters(&positional_params, &ordered_tokens, &mut param_defs)?;
        }
    }

    Ok(Some(param_defs))
}

/// Process named parameters in the format key=value
fn process_named_parameters(
    named_params: &[String],
    ordered_tokens: &IndexSet<String>,
    param_defs: &mut HashMap<String, ParameterDefinition>,
) -> Result<()> {
    for param_str in named_params {
        let parts: Vec<&str> = param_str.split('=').collect();
        if parts.len() != 2 {
            return Err(ParameterFormat(param_str.to_string()));
        }

        let key = parts[0];
        let value = parts[1].to_string();

        if !ordered_tokens.contains(&key.to_string()) {
            return Err(MissingParameter(key.to_string()));
        }

        update_or_create_parameter(param_defs, key, value);
    }
    Ok(())
}

/// Process positional parameters based on token order
fn process_positional_parameters(
    positional_params: &[String],
    ordered_tokens: &IndexSet<String>,
    param_defs: &mut HashMap<String, ParameterDefinition>,
) -> Result<()> {
    // Check if we have enough positional arguments
    if positional_params.len() != ordered_tokens.len() {
        return Err(ParameterCountMismatch(
            ordered_tokens.len(),
            positional_params.len(),
        ));
    }

    // Map positional arguments to tokens
    for (i, token) in ordered_tokens.iter().enumerate() {
        let value = positional_params[i].clone();
        update_or_create_parameter(param_defs, token, value);
    }

    Ok(())
}

/// Update an existing parameter definition or create a new one
fn update_or_create_parameter(
    param_defs: &mut HashMap<String, ParameterDefinition>,
    key: &str,
    value: String,
) {
    if let Some(existing_def) = param_defs.get_mut(key) {
        existing_def.default = Some(value);
    } else {
        // Create a new parameter definition
        let new_def = ParameterDefinition {
            id: key.to_string(),
            description: None,
            default: Some(value),
        };
        param_defs.insert(key.to_string(), new_def);
    }
}
