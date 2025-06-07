//! Argument style determination and validation.
//!
//! This module defines the different styles for providing arguments to commands
//! and validates that only one style is used at a time.

use rust_cuts_core::error::Error::MixedParameterMode;
use rust_cuts_core::error::Result;

/// Represents the style of argument input being used.
///
/// Arguments can be provided in three different ways, and these styles
/// cannot be mixed in a single command invocation.
#[derive(PartialEq, Clone, Debug)]
pub enum Style {
    /// No arguments provided, will use default values or prompt interactively
    None,
    /// Named arguments provided with -p/--param flags (key=value format)
    Named(Vec<String>),
    /// Positional arguments provided as trailing arguments
    Positional(Vec<String>),
}

/// Trait for objects that can determine their argument style.
///
/// This trait provides a standard interface for CLI argument structures
/// to validate and return their argument style.
pub trait Provider {
    /// Validates that named and positional arguments aren't mixed
    /// and returns the appropriate `Style`.
    ///
    /// # Errors
    ///
    /// Returns an error if named and positional arguments are mixed.
    fn get_style(&self) -> Result<Style>;
}

/// Determines the argument style based on provided argument arrays.
///
/// Validates that named and positional arguments are not mixed and returns
/// the appropriate [`Style`].
///
/// # Arguments
///
/// * `named_args` - Array of named arguments in "key=value" format
/// * `positional_args` - Array of positional argument values
///
/// # Returns
///
/// The determined argument style, or an error if both types are provided.
///
/// # Errors
///
/// Returns [`MixedParameterMode`] if both named and positional arguments
/// are provided, as this is not allowed.
///
/// # Examples
///
/// ```rust
/// use rust_cuts_cli::arguments::determine;
///
/// // Named arguments only
/// let style = determine(&["key=value".to_string()], &[]).unwrap();
///
/// // Positional arguments only  
/// let style = determine(&[], &["value1".to_string(), "value2".to_string()]).unwrap();
///
/// // No arguments
/// let style = determine(&[], &[]).unwrap();
/// ```
pub fn determine(
    named_args: &[String],
    positional_args: &[String],
) -> Result<Style> {
    let using_named = !named_args.is_empty();
    let using_positional = !positional_args.is_empty();

    match (using_named, using_positional) {
        (true, true) => Err(MixedParameterMode),
        (true, false) => Ok(Style::Named(named_args.to_vec())),
        (false, true) => Ok(Style::Positional(positional_args.to_vec())),
        (false, false) => Ok(Style::None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_none() {
        let result = determine(&[], &[]).unwrap();
        assert_eq!(result, Style::None);
    }

    #[test]
    fn test_determine_named() {
        let params = vec!["key1=value1".to_string(), "key2=value2".to_string()];
        let result = determine(&params, &[]).unwrap();
        match result {
            Style::Named(values) => {
                assert_eq!(values.len(), 2);
                assert_eq!(values[0], "key1=value1");
                assert_eq!(values[1], "key2=value2");
            }
            _ => panic!("Expected Named argument style"),
        }
    }

    #[test]
    fn test_determine_positional() {
        let args = vec!["value1".to_string(), "value2".to_string()];
        let result = determine(&[], &args).unwrap();
        match result {
            Style::Positional(values) => {
                assert_eq!(values.len(), 2);
                assert_eq!(values[0], "value1");
                assert_eq!(values[1], "value2");
            }
            _ => panic!("Expected Positional argument style"),
        }
    }

    #[test]
    fn test_determine_mixed_error() {
        let params = vec!["key=value".to_string()];
        let args = vec!["positional".to_string()];
        let result = determine(&params, &args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MixedParameterMode));
    }

    #[test]
    fn test_style_equality() {
        assert_eq!(Style::None, Style::None);

        let named1 = Style::Named(vec!["a=1".to_string()]);
        let named2 = Style::Named(vec!["a=1".to_string()]);
        assert_eq!(named1, named2);

        let pos1 = Style::Positional(vec!["value".to_string()]);
        let pos2 = Style::Positional(vec!["value".to_string()]);
        assert_eq!(pos1, pos2);

        // Different styles should not be equal
        assert_ne!(Style::None, named1);
        assert_ne!(named1, pos1);
    }

    #[test]
    fn test_style_debug_formatting() {
        let named = Style::Named(vec!["test=value".to_string()]);
        let debug_str = format!("{named:?}");
        assert!(debug_str.contains("Named"));
        assert!(debug_str.contains("test=value"));
    }
}
