//! Command-line argument parsing and validation.
//!
//! This module defines the command-line interface structure and provides
//! validation for CLI arguments using the `clap` crate.

use crate::arguments::{determine, Provider, Style};
use clap::{Parser, Subcommand};
use rust_cuts_core::error::Result;

/// Main CLI arguments structure for the rust-cuts tool.
///
/// This structure defines the top-level command interface that routes to
/// different subcommands like `exec`, `init`, and `new`.
///
/// # Examples
///
/// ```rust
/// use clap::Parser;
/// use rust_cuts_cli::cli_args::{Args, Commands};
///
/// // Parse arguments from command line with exec subcommand
/// let args = Args::parse_from(["rc", "exec", "my-command"]);
/// match args.command {
///     Commands::Exec(exec_args) => {
///         // Handle exec command
///     }
///     Commands::Init(_) => {
///         // Handle init command  
///     }
///     Commands::New(_) => {
///         // Handle new command
///     }
/// }
/// ```
#[derive(Parser, Debug)]
#[command(
    name = "rc",
    about = "A terminal command management tool",
    term_width = 0
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for the rust-cuts CLI.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute a saved command (default behavior)
    Exec(ExecArgs),
    /// Initialize a new rust-cuts configuration
    Init(InitArgs),
    /// Create a new command definition
    New(NewArgs),
}

/// Arguments for the exec subcommand.
///
/// This contains all the functionality that was previously in the main Args struct.
/// It supports both interactive and direct command execution modes.
///
/// # Examples
///
/// ```bash
/// rc exec my-command
/// rc exec my-command -p key=value
/// rc exec --rerun-last-command
/// ```
#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)] // silence clippy's warning on this struct
pub struct ExecArgs {
    /// Path to the commands definition config file YAML.
    ///
    /// If not provided, defaults to `~/.rust-cuts/commands.yml`.
    #[arg(long, short = 'c')]
    pub config_path: Option<String>,

    /// Path to the file that stores the interpolated last command.
    ///
    /// If not provided, defaults to `~/.rust-cuts/last_command.yml`.
    #[arg(long, short = 'l')]
    pub last_command_path: Option<String>,

    /// Perform a dry run, which just prints out the command but does not execute it.
    ///
    /// When enabled, shows what command would be executed without actually running it.
    #[arg(long, short = 'd', action)]
    pub dry_run: bool,

    /// Run the command without first confirming if the command should be run.
    ///
    /// Skips the interactive confirmation prompt and executes immediately.
    #[arg(long, short = 'f', action)]
    pub force: bool,

    /// Rerun the last command (do not allow to select another).
    ///
    /// Bypasses command selection and immediately reruns the previously executed command.
    #[arg(long, short = 'r', action)]
    pub rerun_last_command: bool,

    /// Skip saving of this command as the last command to replay.
    ///
    /// Prevents overwriting the last command file, retaining the existing last command.
    #[arg(long, short = 's', action)]
    pub skip_command_save: bool,

    /// The command ID or index to execute directly.
    ///
    /// If not provided, interactive mode is used. Can be either:
    /// - A command ID (string identifier)
    /// - A numeric index (0-based position in command list)
    #[arg(num_args(1))]
    pub command_id_or_index: Option<String>,

    /// Named parameters for the command in the format key=value.
    ///
    /// Multiple parameters can be provided with repeated `-p` flags.
    /// Cannot be mixed with positional arguments.
    ///
    /// # Examples
    /// ```bash
    /// rc deploy -p environment=prod -p region=us-west-2
    /// ```
    #[arg(long = "param", short = 'p', action = clap::ArgAction::Append)]
    pub parameters: Vec<String>,

    /// Positional arguments for substitution in the command template.
    ///
    /// Arguments are matched by position to template variables in order of appearance.
    /// Cannot be mixed with named parameters.
    ///
    /// # Examples
    /// ```bash
    /// rc ssh-to prod web-01
    /// ```
    #[arg(trailing_var_arg = true)]
    pub positional_arguments: Vec<String>,
}

/// Arguments for the init subcommand.
///
/// Initializes a new rust-cuts configuration in the current directory.
#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Force initialization even if config already exists
    #[arg(long, short = 'f', action)]
    pub force: bool,

    /// Custom path for the configuration file
    #[arg(long, short = 'c')]
    pub config_path: Option<String>,
}

/// Arguments for the new subcommand.
///
/// Creates a new command definition interactively.
#[derive(Parser, Debug)]
pub struct NewArgs {
    /// ID for the new command
    pub id: Option<String>,

    /// Description for the new command
    #[arg(long, short = 'd')]
    pub description: Option<String>,
}

impl Provider for ExecArgs {
    /// Determines the argument style based on the provided arguments.
    ///
    /// Validates that named and positional arguments aren't mixed and returns
    /// the appropriate [`Style`].
    ///
    /// # Errors
    ///
    /// Returns an error if both named and positional arguments are provided,
    /// as this is not allowed.
    fn get_style(&self) -> Result<Style> {
        determine(&self.parameters, &self.positional_arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_exec_args_default_values() {
        let args = ExecArgs::parse_from(["exec"]);

        assert!(args.config_path.is_none());
        assert!(args.last_command_path.is_none());
        assert!(!args.dry_run);
        assert!(!args.force);
        assert!(!args.rerun_last_command);
        assert!(!args.skip_command_save);
        assert!(args.command_id_or_index.is_none());
        assert!(args.parameters.is_empty());
        assert!(args.positional_arguments.is_empty());
    }

    #[test]
    fn test_exec_args_short_flags() {
        let args = ExecArgs::parse_from([
            "exec",
            "-c",
            "/custom/config.yml",
            "-l",
            "/custom/last.yml",
            "-d",
            "-f",
            "-r",
            "-s",
        ]);

        assert_eq!(args.config_path, Some("/custom/config.yml".to_string()));
        assert_eq!(args.last_command_path, Some("/custom/last.yml".to_string()));
        assert!(args.dry_run);
        assert!(args.force);
        assert!(args.rerun_last_command);
        assert!(args.skip_command_save);
    }

    #[test]
    fn test_exec_args_long_flags() {
        let args = ExecArgs::parse_from([
            "exec",
            "--config-path",
            "/custom/config.yml",
            "--last-command-path",
            "/custom/last.yml",
            "--dry-run",
            "--force",
            "--rerun-last-command",
            "--skip-command-save",
        ]);

        assert_eq!(args.config_path, Some("/custom/config.yml".to_string()));
        assert_eq!(args.last_command_path, Some("/custom/last.yml".to_string()));
        assert!(args.dry_run);
        assert!(args.force);
        assert!(args.rerun_last_command);
        assert!(args.skip_command_save);
    }

    #[test]
    fn test_exec_args_command_id() {
        let args = ExecArgs::parse_from(["exec", "my-command"]);
        assert_eq!(args.command_id_or_index, Some("my-command".to_string()));
    }

    #[test]
    fn test_exec_args_named_parameters() {
        let args = ExecArgs::parse_from([
            "exec",
            "my-command",
            "-p",
            "key1=value1",
            "--param",
            "key2=value2",
        ]);

        assert_eq!(args.command_id_or_index, Some("my-command".to_string()));
        assert_eq!(args.parameters.len(), 2);
        assert_eq!(args.parameters[0], "key1=value1");
        assert_eq!(args.parameters[1], "key2=value2");
    }

    #[test]
    fn test_exec_args_positional_arguments() {
        let args = ExecArgs::parse_from(["exec", "my-command", "--", "pos1", "pos2", "pos3"]);

        assert_eq!(args.command_id_or_index, Some("my-command".to_string()));
        assert_eq!(args.positional_arguments.len(), 3);
        assert_eq!(args.positional_arguments[0], "pos1");
        assert_eq!(args.positional_arguments[1], "pos2");
        assert_eq!(args.positional_arguments[2], "pos3");
    }

    #[test]
    fn test_style_provider_none() {
        let args = ExecArgs::parse_from(["exec"]);
        let style = args.get_style().unwrap();
        assert_eq!(style, Style::None);
    }

    #[test]
    fn test_style_provider_named() {
        let args = ExecArgs::parse_from(["exec", "-p", "key=value"]);
        let style = args.get_style().unwrap();
        match style {
            Style::Named(params) => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0], "key=value");
            }
            _ => panic!("Expected Named argument style"),
        }
    }

    #[test]
    fn test_style_provider_positional() {
        // With trailing_var_arg, first arg goes to command_id_or_index, rest to positional_arguments
        let args = ExecArgs::parse_from(["exec", "command", "value1", "value2"]);
        let style = args.get_style().unwrap();
        match style {
            Style::Positional(arguments) => {
                assert_eq!(arguments.len(), 2);
                assert_eq!(arguments[0], "value1");
                assert_eq!(arguments[1], "value2");
            }
            _ => panic!("Expected Positional argument style"),
        }
        // Verify the first arg went to command_id_or_index
        assert_eq!(args.command_id_or_index, Some("command".to_string()));
    }

    #[test]
    fn test_style_provider_mixed_error() {
        // This creates a mixed mode scenario that should error
        let args = ExecArgs {
            config_path: None,
            last_command_path: None,
            dry_run: false,
            force: false,
            rerun_last_command: false,
            skip_command_save: false,
            command_id_or_index: None,
            parameters: vec!["key=value".to_string()],
            positional_arguments: vec!["positional".to_string()],
        };
        let result = args.get_style();
        assert!(result.is_err());
    }

    #[test]
    fn test_subcommands_parsing() {
        // Test exec subcommand
        let args = Args::parse_from(["rc", "exec", "my-command"]);
        match args.command {
            Commands::Exec(exec_args) => {
                assert_eq!(
                    exec_args.command_id_or_index,
                    Some("my-command".to_string())
                );
            }
            _ => panic!("Expected Exec command"),
        }

        // Test init subcommand
        let args = Args::parse_from(["rc", "init", "--force"]);
        match args.command {
            Commands::Init(init_args) => {
                assert!(init_args.force);
            }
            _ => panic!("Expected Init command"),
        }

        // Test new subcommand
        let args = Args::parse_from(["rc", "new", "test-command"]);
        match args.command {
            Commands::New(new_args) => {
                assert_eq!(new_args.id, Some("test-command".to_string()));
            }
            _ => panic!("Expected New command"),
        }
    }
}
