use indexmap::IndexSet;
use std::collections::HashMap;

use rust_cuts_core::command_definitions::ParameterDefinition;
use crate::parameters::ParameterMode;

/// Determines whether to prompt the user for parameter values.
///
/// The function avoids prompting when:
/// 1. The command template contains no tokens to interpolate
/// 2. The user reruns a command (which already has all required parameters)
/// 3. The user supplies all needed parameters via command-line (named or positional)
///
/// The function prompts when:
/// 1. The user runs a new command without providing command-line parameters
/// 2. Command-line parameters don't cover all required tokens
///
/// Note: For reruns, we assume the last command already has all parameters it needs.
/// If parameters were removed from the previous execution, they simply won't be used.
pub fn should_prompt_for_parameters(
    tokens: &IndexSet<String>,
    filled_parameters: &Option<HashMap<String, ParameterDefinition>>,
    is_rerun: bool,
    parameter_mode: &ParameterMode,
) -> bool {
    // No need to prompt if no tokens to fill
    if tokens.is_empty() {
        return false;
    }

    // No need to prompt for reruns - they already have all parameters
    if is_rerun {
        return false;
    }

    // If using command-line parameters (Named or Positional), check if any are missing
    if *parameter_mode != ParameterMode::None {
        if let Some(params) = filled_parameters {
            return has_missing_token_values(tokens, params);
        }
    }

    // For new commands with no command-line parameters, we need to prompt
    true
}

/// Check if any tokens are missing values in the parameter definitions
fn has_missing_token_values(tokens: &IndexSet<String>, params: &HashMap<String, ParameterDefinition>) -> bool {
    tokens.iter().any(|token| {
        match params.get(token) {
            Some(param) => param.default.is_none(),
            None => true, // Token has no parameter definition
        }
    })
}