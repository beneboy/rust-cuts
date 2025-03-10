use rust_cuts_core::error::Error::MixedParameterMode;
use rust_cuts_core::error::Result;

/// Represents the mode of parameter input being used
#[derive(PartialEq, Clone, Debug)]
pub enum ParameterMode {
    /// No parameters provided, will use default values or prompt interactively
    None,
    /// Named parameters provided with -p/--param flags (key=value format)
    Named(Vec<String>),
    /// Positional parameters provided as trailing arguments
    Positional(Vec<String>),
}

/// Helper functions for CLI args to determine parameter mode
pub trait ParameterModeProvider {
    /// Validates that named and positional parameters aren't mixed
    /// and returns the appropriate ParameterMode
    fn get_parameter_mode(&self) -> Result<ParameterMode>;
}

/// Implementation for a struct that has parameters and positional_args fields
pub fn determine_parameter_mode(parameters: &[String], positional_args: &[String]) -> Result<ParameterMode> {
    let using_named = !parameters.is_empty();
    let using_positional = !positional_args.is_empty();

    match (using_named, using_positional) {
        (true, true) => Err(MixedParameterMode),
        (true, false) => Ok(ParameterMode::Named(parameters.to_vec())),
        (false, true) => Ok(ParameterMode::Positional(positional_args.to_vec())),
        (false, false) => Ok(ParameterMode::None),
    }
}