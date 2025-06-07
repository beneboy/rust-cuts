use indexmap::IndexSet;
use std::collections::HashMap;

use crate::arguments::Style;
use rust_cuts_core::command_definitions::ParameterDefinition;

/// Determines whether to prompt the user for parameter values.
///
/// The function avoids prompting when:
/// 1. The command template contains no tokens to interpolate
/// 2. The user reruns a command (which already has all required parameters)
///
/// The function prompts when:
/// 1. The user runs a new command (always prompts, even with defaults)
///
/// Note: When running a rerun, the user can still choose to change parameters
/// during the run confirmation using the 'c' option.
#[must_use]
pub fn should_prompt_for_parameters<S: std::hash::BuildHasher>(
    tokens: &IndexSet<String>,
    filled_parameters: Option<&HashMap<String, ParameterDefinition, S>>,
    is_rerun: bool,
    argument_style: &Style,
) -> bool {
    // No need to prompt if no tokens to fill
    if tokens.is_empty() {
        return false;
    }

    // No need to prompt for reruns - they already have all parameters
    // Users can use the 'c' option during confirmation if they want to change parameters
    if is_rerun {
        return false;
    }

    // For command-line arguments (Named or Positional), we only skip prompting
    // if the user has provided ALL parameters via the command line
    if *argument_style != Style::None {
        if let Some(params) = filled_parameters {
            // Only skip prompting if every token has a command-line value
            return !has_all_command_line_parameters(tokens, params);
        }
    }

    // For new commands, we always prompt (regardless of defaults)
    true
}

/// Check if all tokens have parameter values explicitly provided via command line
/// This is different from having defaults - we only want to skip prompting
/// if the user has explicitly provided ALL values via command line
fn has_all_command_line_parameters<S: std::hash::BuildHasher>(
    tokens: &IndexSet<String>,
    params: &HashMap<String, ParameterDefinition, S>,
) -> bool {
    tokens.iter().all(|token| params.get(token).is_some())
}
