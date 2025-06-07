//! Rust Cuts CLI Library
//!
//! This crate provides the command-line interface for rust-cuts, a terminal command
//! management tool. It handles user interaction, command selection, parameter input,
//! and command execution workflows.
//!
//! # Key Features
//!
//! - **Interactive Command Selection**: Terminal-based UI for browsing and selecting commands
//! - **Parameter Processing**: Support for named parameters and positional argument input
//! - **Command Confirmation**: Interactive confirmation before command execution
//! - **Flexible Parameter Modes**: Support for command-line parameters or interactive prompts
//! - **Command History**: Ability to rerun the last executed command
//!
//! # Architecture
//!
//! The CLI is organized into several key modules:
//!
//! - [`cli_args`]: Command-line argument parsing and validation
//! - [`command_selection`]: Interactive UI for command selection and confirmation
//! - [`arguments`]: Argument processing for both named and positional styles
//!
//! # Examples
//!
//! The CLI binary (`rc`) can be used in several ways:
//!
//! ```bash
//! # Interactive mode - shows command selection UI
//! rc
//!
//! # Direct command execution by ID
//! rc my-command
//!
//! # With named parameters
//! rc my-command --param host=localhost --param port=8080
//!
//! # With positional arguments  
//! rc my-command localhost 8080
//!
//! # Rerun last command
//! rc --rerun-last-command
//!
//! # Dry run (don't execute, just show what would run)
//! rc --dry-run my-command
//! ```

pub mod arguments;
pub mod cli_args;
pub mod command_selection;
