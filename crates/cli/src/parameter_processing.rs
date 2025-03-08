use crate::cli_args::ParameterMode;
use rust_cuts_core::command_definitions::{
    CommandExecutionTemplate, ParameterDefinition, TemplateParser,
};
use rust_cuts_core::error::Error::{MissingParameter, ParameterCountMismatch, ParameterFormat};
use rust_cuts_core::error::Result;
use std::collections::HashMap;

pub fn process_command_line_parameters(
    parameter_mode: ParameterMode,
    execution_context: &CommandExecutionTemplate,
    parameter_definitions: &Option<HashMap<String, ParameterDefinition>>,
) -> Result<Option<HashMap<String, ParameterDefinition>>> {
    let ordered_tokens = execution_context.get_ordered_context_variables()?;

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
            }
        }

        ParameterMode::Named(named_params) => {
            // Process named parameters (key=value format)
            for param_str in named_params {
                let parts: Vec<&str> = param_str.split('=').collect();
                if parts.len() != 2 {
                    return Err(ParameterFormat(param_str.to_string()));
                }

                let key = parts[0];
                let value = parts[1].to_string();

                if !ordered_tokens.contains(key) {
                    return Err(MissingParameter(key.to_string()));
                }

                // Update or create parameter definition
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
        }

        ParameterMode::Positional(positional_params) => {
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

                // Update or create parameter definition
                if let Some(existing_def) = param_defs.get_mut(token) {
                    existing_def.default = Some(value);
                } else {
                    // Create a new parameter definition
                    let new_def = ParameterDefinition {
                        id: token.clone(),
                        description: None,
                        default: Some(value),
                    };
                    param_defs.insert(token.clone(), new_def);
                }
            }
        }
    }

    Ok(Some(param_defs))
}
