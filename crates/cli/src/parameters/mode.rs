//! Parameter mode determination and validation.
//!
//! This module defines the different modes for providing parameters to commands
//! and validates that only one mode is used at a time.

use rust_cuts_core::error::Error::MixedParameterMode;
use rust_cuts_core::error::Result;

/// Represents the mode of parameter input being used.
///
/// Parameters can be provided in three different ways, and these modes
/// cannot be mixed in a single command invocation.
#[derive(PartialEq, Clone, Debug)]
pub enum ParameterMode {
    /// No parameters provided, will use default values or prompt interactively
    None,
    /// Named parameters provided with -p/--param flags (key=value format)
    Named(Vec<String>),
    /// Positional parameters provided as trailing arguments
    Positional(Vec<String>),
}

/// Trait for objects that can determine their parameter mode.
///
/// This trait provides a standard interface for CLI argument structures
/// to validate and return their parameter mode.
pub trait ParameterModeProvider {
    /// Validates that named and positional parameters aren't mixed
    /// and returns the appropriate `ParameterMode`.
    ///
    /// # Errors
    ///
    /// Returns an error if named and positional parameters are mixed.
    fn get_parameter_mode(&self) -> Result<ParameterMode>;
}

/// Determines the parameter mode based on provided parameter arrays.
///
/// Validates that named and positional parameters are not mixed and returns
/// the appropriate [`ParameterMode`].
///
/// # Arguments
///
/// * `parameters` - Array of named parameters in "key=value" format
/// * `positional_args` - Array of positional parameter values
///
/// # Returns
///
/// The determined parameter mode, or an error if both types are provided.
///
/// # Errors
///
/// Returns [`MixedParameterMode`] if both named and positional parameters
/// are provided, as this is not allowed.
///
/// # Examples
///
/// ```rust
/// use rust_cuts_cli::parameters::determine_parameter_mode;
///
/// // Named parameters only
/// let mode = determine_parameter_mode(&["key=value".to_string()], &[]).unwrap();
///
/// // Positional parameters only  
/// let mode = determine_parameter_mode(&[], &["value1".to_string(), "value2".to_string()]).unwrap();
///
/// // No parameters
/// let mode = determine_parameter_mode(&[], &[]).unwrap();
/// ```
pub fn determine_parameter_mode(
    parameters: &[String],
    positional_args: &[String],
) -> Result<ParameterMode> {
    let using_named = !parameters.is_empty();
    let using_positional = !positional_args.is_empty();

    match (using_named, using_positional) {
        (true, true) => Err(MixedParameterMode),
        (true, false) => Ok(ParameterMode::Named(parameters.to_vec())),
        (false, true) => Ok(ParameterMode::Positional(positional_args.to_vec())),
        (false, false) => Ok(ParameterMode::None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_parameter_mode_none() {
        let result = determine_parameter_mode(&[], &[]).unwrap();
        assert_eq!(result, ParameterMode::None);
    }

    #[test]
    fn test_determine_parameter_mode_named() {
        let params = vec!["key1=value1".to_string(), "key2=value2".to_string()];
        let result = determine_parameter_mode(&params, &[]).unwrap();
        match result {
            ParameterMode::Named(values) => {
                assert_eq!(values.len(), 2);
                assert_eq!(values[0], "key1=value1");
                assert_eq!(values[1], "key2=value2");
            }
            _ => panic!("Expected Named parameter mode"),
        }
    }

    #[test]
    fn test_determine_parameter_mode_positional() {
        let args = vec!["value1".to_string(), "value2".to_string()];
        let result = determine_parameter_mode(&[], &args).unwrap();
        match result {
            ParameterMode::Positional(values) => {
                assert_eq!(values.len(), 2);
                assert_eq!(values[0], "value1");
                assert_eq!(values[1], "value2");
            }
            _ => panic!("Expected Positional parameter mode"),
        }
    }

    #[test]
    fn test_determine_parameter_mode_mixed_error() {
        let params = vec!["key=value".to_string()];
        let args = vec!["positional".to_string()];
        let result = determine_parameter_mode(&params, &args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MixedParameterMode));
    }

    #[test]
    fn test_parameter_mode_equality() {
        assert_eq!(ParameterMode::None, ParameterMode::None);

        let named1 = ParameterMode::Named(vec!["a=1".to_string()]);
        let named2 = ParameterMode::Named(vec!["a=1".to_string()]);
        assert_eq!(named1, named2);

        let pos1 = ParameterMode::Positional(vec!["value".to_string()]);
        let pos2 = ParameterMode::Positional(vec!["value".to_string()]);
        assert_eq!(pos1, pos2);

        // Different modes should not be equal
        assert_ne!(ParameterMode::None, named1);
        assert_ne!(named1, pos1);
    }

    #[test]
    fn test_parameter_mode_debug_formatting() {
        let named = ParameterMode::Named(vec!["test=value".to_string()]);
        let debug_str = format!("{named:?}");
        assert!(debug_str.contains("Named"));
        assert!(debug_str.contains("test=value"));
    }
}
