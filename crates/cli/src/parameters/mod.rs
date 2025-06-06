//! Parameter processing and validation for rust-cuts CLI.
//!
//! This module handles the different ways parameters can be provided to commands:
//! - **Named parameters**: Using `-p key=value` format
//! - **Positional parameters**: Provided as trailing arguments
//! - **Interactive prompts**: When no parameters are provided via CLI
//!
//! The module ensures parameter modes cannot be mixed and provides validation
//! for parameter formats and values.

// Export public items from submodules
pub mod mode;
pub mod processing;
pub mod validation;

// Re-exports for convenience
pub use mode::determine_parameter_mode;
pub use mode::ParameterMode;
pub use mode::ParameterModeProvider;
pub use processing::process_command_line_parameters;
