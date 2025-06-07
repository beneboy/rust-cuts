//! Argument processing and validation for rust-cuts CLI.
//!
//! This module handles the different ways arguments can be provided to commands:
//! - **Named arguments**: Using `-p key=value` format
//! - **Positional arguments**: Provided as trailing arguments
//! - **Interactive prompts**: When no arguments are provided via CLI
//!
//! The module ensures argument styles cannot be mixed and provides validation
//! for argument formats and values.

// Export public items from submodules
pub mod processing;
pub mod style;
pub mod validation;

// Re-exports for convenience
pub use processing::process_command_line;
pub use style::determine;
pub use style::Provider;
pub use style::Style;
