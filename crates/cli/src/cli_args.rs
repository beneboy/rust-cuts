//! Command-line argument parsing and validation.
//!
//! This module defines the command-line interface structure and provides
//! validation for CLI arguments using the `clap` crate.

use crate::parameters::{determine_parameter_mode, ParameterMode, ParameterModeProvider};
use clap::Parser;
use rust_cuts_core::error::Result;

/// Command-line arguments for the rust-cuts CLI tool.
///
/// This structure defines all available command-line options and arguments
/// that can be passed to the `rc` binary. It supports both interactive and
/// direct command execution modes.
///
/// # Examples
///
/// ```rust
/// use clap::Parser;
/// use rust_cuts_cli::cli_args::Args;
///
/// // Parse arguments from command line
/// let args = Args::parse();
/// ```
#[derive(Parser, Debug)] // requires `derive` feature
#[command(term_width = 0)] // Just to make testing across clap features easier
#[allow(clippy::struct_excessive_bools)] // silence clippy's warning on this struct
pub struct Args {
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
    /// Cannot be mixed with positional parameters.
    ///
    /// # Examples
    /// ```bash
    /// rc deploy -p environment=prod -p region=us-west-2
    /// ```
    #[arg(long = "param", short = 'p', action = clap::ArgAction::Append)]
    pub parameters: Vec<String>,

    /// Positional parameters for substitution in the command template.
    ///
    /// Parameters are matched by position to template variables in order of appearance.
    /// Cannot be mixed with named parameters.
    ///
    /// # Examples
    /// ```bash
    /// rc ssh-to prod web-01
    /// ```
    #[arg(trailing_var_arg = true)]
    pub positional_args: Vec<String>,
}

impl ParameterModeProvider for Args {
    /// Determines the parameter mode based on the provided arguments.
    ///
    /// Validates that named and positional parameters aren't mixed and returns
    /// the appropriate [`ParameterMode`].
    ///
    /// # Errors
    ///
    /// Returns an error if both named and positional parameters are provided,
    /// as this is not allowed.
    fn get_parameter_mode(&self) -> Result<ParameterMode> {
        determine_parameter_mode(&self.parameters, &self.positional_args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_args_default_values() {
        let args = Args::parse_from(["rc"]);

        assert!(args.config_path.is_none());
        assert!(args.last_command_path.is_none());
        assert!(!args.dry_run);
        assert!(!args.force);
        assert!(!args.rerun_last_command);
        assert!(!args.skip_command_save);
        assert!(args.command_id_or_index.is_none());
        assert!(args.parameters.is_empty());
        assert!(args.positional_args.is_empty());
    }

    #[test]
    fn test_args_short_flags() {
        let args = Args::parse_from([
            "rc",
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
    fn test_args_long_flags() {
        let args = Args::parse_from([
            "rc",
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
    fn test_args_command_id() {
        let args = Args::parse_from(["rc", "my-command"]);
        assert_eq!(args.command_id_or_index, Some("my-command".to_string()));
    }

    #[test]
    fn test_args_named_parameters() {
        let args = Args::parse_from([
            "rc",
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
    fn test_args_positional_parameters() {
        let args = Args::parse_from(["rc", "my-command", "--", "pos1", "pos2", "pos3"]);

        assert_eq!(args.command_id_or_index, Some("my-command".to_string()));
        assert_eq!(args.positional_args.len(), 3);
        assert_eq!(args.positional_args[0], "pos1");
        assert_eq!(args.positional_args[1], "pos2");
        assert_eq!(args.positional_args[2], "pos3");
    }

    #[test]
    fn test_parameter_mode_provider_none() {
        let args = Args::parse_from(["rc"]);
        let mode = args.get_parameter_mode().unwrap();
        assert_eq!(mode, ParameterMode::None);
    }

    #[test]
    fn test_parameter_mode_provider_named() {
        let args = Args::parse_from(["rc", "-p", "key=value"]);
        let mode = args.get_parameter_mode().unwrap();
        match mode {
            ParameterMode::Named(params) => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0], "key=value");
            }
            _ => panic!("Expected Named parameter mode"),
        }
    }

    #[test]
    fn test_parameter_mode_provider_positional() {
        // With trailing_var_arg, first arg goes to command_id_or_index, rest to positional_args
        let args = Args::parse_from(["rc", "command", "value1", "value2"]);
        let mode = args.get_parameter_mode().unwrap();
        match mode {
            ParameterMode::Positional(params) => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], "value1");
                assert_eq!(params[1], "value2");
            }
            _ => panic!("Expected Positional parameter mode"),
        }
        // Verify the first arg went to command_id_or_index
        assert_eq!(args.command_id_or_index, Some("command".to_string()));
    }

    #[test]
    fn test_parameter_mode_provider_mixed_error() {
        // This creates a mixed mode scenario that should error
        let args = Args {
            config_path: None,
            last_command_path: None,
            dry_run: false,
            force: false,
            rerun_last_command: false,
            skip_command_save: false,
            command_id_or_index: None,
            parameters: vec!["key=value".to_string()],
            positional_args: vec!["positional".to_string()],
        };
        let result = args.get_parameter_mode();
        assert!(result.is_err());
    }
}
