//! Configuration path utilities for rust-cuts.
//!
//! This module provides functions for resolving configuration file paths
//! and expanding shell variables like `~` in paths.

/// Default path for the commands configuration file
const DEFAULT_CONFIG_PATH: &str = "~/.rust-cuts/commands.yml";
/// Default path for storing the last executed command
const DEFAULT_LAST_COMMAND_PATH: &str = "~/.rust-cuts/last_command.yml";

/// Default shell to use for command execution
pub const DEFAULT_SHELL: &str = "/bin/bash";

/// Resolves the configuration file path.
///
/// If a custom path is provided, uses that path. Otherwise, uses the default
/// configuration path. Shell expansions like `~` are resolved.
///
/// # Arguments
///
/// * `config_path_arg` - Optional custom configuration file path
///
/// # Returns
///
/// The resolved absolute path to the configuration file
///
/// # Examples
///
/// ```
/// use rust_cuts_core::config::get_config_path;
///
/// // Use default path
/// let default_path = get_config_path(&None);
///
/// // Use custom path
/// let custom_path = get_config_path(&Some("/path/to/config.yml".to_string()));
/// ```
pub fn get_config_path(config_path_arg: &Option<String>) -> String {
    let config_path = match config_path_arg {
        Some(last_command_path) => last_command_path,
        None => DEFAULT_CONFIG_PATH,
    };

    shellexpand::tilde(config_path).to_string()
}

/// Resolves the last command file path.
///
/// If a custom path is provided, uses that path. Otherwise, uses the default
/// last command path. Shell expansions like `~` are resolved.
///
/// # Arguments
///
/// * `last_command_path_arg` - Optional custom last command file path
///
/// # Returns
///
/// The resolved absolute path to the last command file
pub fn get_last_command_path(last_command_path_arg: &Option<String>) -> String {
    let last_command_path = match last_command_path_arg {
        Some(last_command_path) => last_command_path,
        None => DEFAULT_LAST_COMMAND_PATH,
    };

    shellexpand::tilde(last_command_path).to_string()
}

/// Expands shell variables in a working directory path.
///
/// If a working directory is provided, expands shell variables like `~`.
/// Returns None if no working directory is provided.
///
/// # Arguments
///
/// * `working_directory` - Optional working directory path that may contain shell variables
///
/// # Returns
///
/// The expanded working directory path, or None if input is None
///
/// # Examples
///
/// ```
/// use rust_cuts_core::config::expand_working_directory;
///
/// // Expand tilde
/// let expanded = expand_working_directory(&Some("~/projects".to_string()));
/// assert!(expanded.is_some());
///
/// // Handle None input
/// let none_result = expand_working_directory(&None);
/// assert!(none_result.is_none());
/// ```
pub fn expand_working_directory(working_directory: &Option<String>) -> Option<String> {
    if let Some(working_directory) = working_directory {
        return Some({
            let expanded = shellexpand::tilde(working_directory);
            expanded.to_string()
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_path_with_custom_path() {
        let custom_path = Some("/custom/path/config.yml".to_string());
        let result = get_config_path(&custom_path);
        assert_eq!(result, "/custom/path/config.yml");
    }

    #[test]
    fn test_get_config_path_with_none() {
        let result = get_config_path(&None);
        // Should expand the tilde in the default path
        assert!(result.contains("commands.yml"));
        assert!(!result.starts_with('~'));
    }

    #[test]
    fn test_get_config_path_with_tilde() {
        let tilde_path = Some("~/my-config.yml".to_string());
        let result = get_config_path(&tilde_path);
        // Should expand the tilde
        assert!(!result.starts_with('~'));
        assert!(result.ends_with("my-config.yml"));
    }

    #[test]
    fn test_get_last_command_path_with_custom_path() {
        let custom_path = Some("/custom/last_command.yml".to_string());
        let result = get_last_command_path(&custom_path);
        assert_eq!(result, "/custom/last_command.yml");
    }

    #[test]
    fn test_get_last_command_path_with_none() {
        let result = get_last_command_path(&None);
        // Should expand the tilde in the default path
        assert!(result.contains("last_command.yml"));
        assert!(!result.starts_with('~'));
    }

    #[test]
    fn test_expand_working_directory_with_some() {
        let working_dir = Some("~/projects/rust-cuts".to_string());
        let result = expand_working_directory(&working_dir);

        assert!(result.is_some());
        let expanded = result.unwrap();
        assert!(!expanded.starts_with('~'));
        assert!(expanded.ends_with("projects/rust-cuts"));
    }

    #[test]
    fn test_expand_working_directory_with_none() {
        let result = expand_working_directory(&None);
        assert!(result.is_none());
    }

    #[test]
    fn test_expand_working_directory_without_tilde() {
        let working_dir = Some("/absolute/path".to_string());
        let result = expand_working_directory(&working_dir);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "/absolute/path");
    }

    #[test]
    fn test_default_shell_constant() {
        assert_eq!(DEFAULT_SHELL, "/bin/bash");
    }
}
