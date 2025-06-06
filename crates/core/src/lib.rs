//! Rust Cuts Core Library
//!
//! This crate provides the core functionality for rust-cuts, a terminal command
//! management tool that allows users to save and execute templated commands with
//! parameters, working directories, and environment variables.
//!
//! # Key Features
//!
//! - **Command Definitions**: Parse and validate YAML-based command configurations
//! - **Parameter Templating**: Support for templated commands with user-provided values
//! - **Execution Context**: Working directory and environment variable management
//! - **Configuration Management**: Handle configuration file paths and settings
//! - **Error Handling**: Comprehensive error types for all failure modes
//!
//! # Examples
//!
//! Loading command definitions from a configuration file:
//!
//! ```no_run
//! use rust_cuts_core::file_handling::get_command_definitions;
//!
//! let commands = get_command_definitions(&"~/.rust-cuts/commands.yml".to_string())?;
//! for command in &commands {
//!     println!("Command: {}", command);
//! }
//! # Ok::<(), rust_cuts_core::error::Error>(())
//! ```

pub mod command_definitions;
pub mod config;
pub mod error;
pub mod execution;
pub mod file_handling;
pub mod interpolation;
