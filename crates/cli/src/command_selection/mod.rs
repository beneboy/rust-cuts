//! Interactive command selection and user input handling.
//!
//! This module provides the terminal-based user interface for rust-cuts,
//! including command selection, parameter input, and confirmation dialogs.
//!
//! # Key Features
//!
//! - **Interactive Command List**: Scrollable list of available commands
//! - **Fuzzy Search**: Filter commands by typing to search
//! - **Parameter Prompting**: Interactive input for command parameters
//! - **Command Confirmation**: Confirmation dialog before execution
//! - **Keyboard Navigation**: Full keyboard control with mouse support
//!
//! # User Interface
//!
//! The interface supports:
//! - Arrow keys or vim-style (j/k) navigation
//! - Enter to select a command
//! - Typing to filter commands (fuzzy search)
//! - 'r' to rerun the last command
//! - 'q' or Escape to quit

// Export public items from submodules
pub mod colors;
pub mod input;
pub mod types;
pub mod ui;

// Re-exports for convenience
pub use input::{confirm_command_should_run, fill_parameter_values, prompt_value};
pub use types::{CommandChoice, RunChoice};
pub use ui::prompt_for_command_choice;

/// Character used to select the "rerun last command" option
pub const LAST_COMMAND_OPTION: char = 'r';
