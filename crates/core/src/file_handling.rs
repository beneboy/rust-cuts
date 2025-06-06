//! File handling and validation for rust-cuts configuration.
//!
//! This module provides functions for reading and writing command definitions
//! and last command state, along with validation of command and parameter IDs.

use std::collections::HashSet;
use std::fs::File;
use std::path::Path;

use crate::command_definitions::{
    CommandDefinition, CommandExecutionTemplate, ParameterDefinition, TemplateParser,
};
use crate::error::Error::{
    EmptyId, IdWithColon, IdWithSpace, NonUniqueCommandId, NonUniqueParameterId,
    NotFoundParameterId, NumericId,
};
use crate::error::{Error, Result};

fn get_reader(file_description: &str, path: &str) -> Result<File> {
    match File::open(path) {
        Ok(reader) => Ok(reader),
        Err(e) => Err(Error::io_error(
            file_description.to_string(),
            path.to_string(),
            e,
        )),
    }
}

fn get_last_command_reader(last_command_path: &String) -> Result<Option<File>> {
    if !Path::exists(Path::new(last_command_path)) {
        return Ok(None);
    }

    match get_reader("last command", last_command_path) {
        Ok(f) => Ok(Some(f)),
        Err(e) => Err(e),
    }
}

/// Reads the last executed command from disk.
///
/// Attempts to read and deserialize the last command from the specified path.
/// Returns None if the file doesn't exist.
///
/// # Arguments
///
/// * `last_command_path` - Path to the last command file
///
/// # Returns
///
/// The last command template if it exists and can be read, None if the file
/// doesn't exist, or an error if reading/parsing fails.
///
/// # Errors
///
/// Returns an error if:
/// - The file exists but cannot be read
/// - The file contains invalid YAML
/// - The YAML doesn't match the expected structure
pub fn get_last_command(last_command_path: &String) -> Result<Option<CommandExecutionTemplate>> {
    let last_command_reader = get_last_command_reader(last_command_path)?;
    let Some(last_command_reader) = last_command_reader else {
        return Ok(None);
    };

    // This can't be shortcut with ? as there is an error/some confusion with serde wanting to deserialize the error
    let last_command_parameter: serde_yaml::Result<CommandExecutionTemplate> =
        serde_yaml::from_reader(last_command_reader);

    match last_command_parameter {
        Ok(last_command_parameter) => Ok(Some(last_command_parameter)),
        Err(e) => Err(Error::yaml_error(
            "reading".to_string(),
            "last command".to_string(),
            last_command_path.to_string(),
            e,
        )),
    }
}

/// Writes the last executed command to disk.
///
/// Serializes and saves the command execution template to the specified path
/// for later retrieval.
///
/// # Arguments
///
/// * `path` - Path where to save the last command
/// * `last_command` - The command execution template to save
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be created or written to
/// - Serialization to YAML fails
pub fn write_last_command(path: &str, last_command: &CommandExecutionTemplate) -> Result<()> {
    let f = File::create(path);

    let Ok(f) = f else {
        return Err(Error::io_error(
            "last command".to_string(),
            path.to_string(),
            f.unwrap_err(),
        ));
    };

    serde_yaml::to_writer(f, &last_command).map_err(|e| {
        Error::yaml_error(
            path.to_string(),
            "writing".to_string(),
            "last command".to_string(),
            e,
        )
    })
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(EmptyId);
    }

    if id.contains(' ') {
        return Err(IdWithSpace(id.to_string()));
    }

    if id.contains(':') {
        return Err(IdWithColon(id.to_string()));
    }

    if id.chars().all(|c| c.is_numeric()) {
        return Err(NumericId(id.to_string()));
    }

    Ok(())
}

fn validate_parameters(
    command: &CommandDefinition,
    parameters: &[ParameterDefinition],
) -> Result<()> {
    let mut ids = HashSet::new();
    for parameter in parameters.iter() {
        validate_id(&parameter.id)?;

        if !ids.insert(parameter.id.clone()) {
            // Found a duplicate ID
            return Err(NonUniqueParameterId(
                format!("{command}"),
                parameter.id.clone(),
            ));
        }
    }

    let command_variables = command.get_ordered_context_variables()?;

    for id in ids.iter() {
        if !command_variables.contains(id) {
            return Err(NotFoundParameterId(format!("{command}"), id.clone()));
        }
    }

    Ok(())
}

fn validate_command_ids(commands: &[CommandDefinition]) -> Result<()> {
    let mut ids = HashSet::new();

    for cmd in commands.iter() {
        if let Some(id) = &cmd.id {
            validate_id(id)?;

            if !ids.insert(id.clone()) {
                // Found a duplicate ID
                return Err(NonUniqueCommandId(id.clone()));
            }
        }

        if let Some(parameters) = &cmd.parameters {
            validate_parameters(cmd, parameters)?;
        }
    }

    Ok(())
}

/// Loads and validates command definitions from a configuration file.
///
/// Reads the YAML configuration file, parses command definitions, and validates
/// that all command and parameter IDs are unique and properly formatted.
///
/// # Arguments
///
/// * `config_path` - Path to the YAML configuration file
///
/// # Returns
///
/// A vector of validated command definitions
///
/// # Errors
///
/// Returns an error if:
/// - The configuration file cannot be read
/// - The YAML is malformed or doesn't match the expected structure
/// - The configuration file is empty
/// - Command or parameter IDs are invalid or non-unique
/// - Parameters reference template variables that don't exist in commands
///
/// # Examples
///
/// ```no_run
/// use rust_cuts_core::file_handling::get_command_definitions;
///
/// let commands = get_command_definitions(&"~/.rust-cuts/commands.yml".to_string())?;
/// println!("Loaded {} commands", commands.len());
/// # Ok::<(), rust_cuts_core::error::Error>(())
/// ```
pub fn get_command_definitions(config_path: &String) -> Result<Vec<CommandDefinition>> {
    let config_reader = &get_reader("config", config_path)?;

    let parsing_result: serde_yaml::Result<Vec<CommandDefinition>>;

    {
        parsing_result = serde_yaml::from_reader(config_reader);
    }

    let parsed_command_defs = parsing_result.map_err(|e| {
        Error::yaml_error(
            config_path.clone(),
            "reading".to_string(),
            "config".to_string(),
            e,
        )
    })?;

    if parsed_command_defs.is_empty() {
        return Err(Error::empty_command_definition(config_path.to_string()));
    }

    validate_command_ids(&parsed_command_defs)?;

    Ok(parsed_command_defs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_command_with_id(id: &str) -> CommandDefinition {
        CommandDefinition {
            command: vec!["echo".to_string(), "test".to_string()],
            id: Some(id.to_string()),
            description: None,
            working_directory: None,
            parameters: None,
            environment: None,
            metadata: None,
        }
    }

    fn create_test_command_with_param(param_id: &str, cmd_template: &str) -> CommandDefinition {
        CommandDefinition {
            command: vec!["echo".to_string(), cmd_template.to_string()],
            id: Some("test_cmd".to_string()),
            description: None,
            working_directory: None,
            parameters: Some(vec![ParameterDefinition {
                id: param_id.to_string(),
                default: None,
                description: None,
            }]),
            environment: None,
            metadata: None,
        }
    }

    #[test]
    fn test_validate_id_valid() {
        assert!(validate_id("valid_id").is_ok());
        assert!(validate_id("test123").is_ok());
        assert!(validate_id("my-command").is_ok());
        assert!(validate_id("_underscore").is_ok());
    }

    #[test]
    fn test_validate_id_empty() {
        let result = validate_id("");
        assert!(matches!(result, Err(EmptyId)));
    }

    #[test]
    fn test_validate_id_with_space() {
        let result = validate_id("has space");
        assert!(matches!(result, Err(IdWithSpace(_))));
    }

    #[test]
    fn test_validate_id_with_colon() {
        let result = validate_id("has:colon");
        assert!(matches!(result, Err(IdWithColon(_))));
    }

    #[test]
    fn test_validate_id_numeric_only() {
        let result = validate_id("123");
        assert!(matches!(result, Err(NumericId(_))));
    }

    #[test]
    fn test_validate_command_ids_unique() {
        let commands = vec![
            create_test_command_with_id("cmd1"),
            create_test_command_with_id("cmd2"),
            create_test_command_with_id("cmd3"),
        ];
        assert!(validate_command_ids(&commands).is_ok());
    }

    #[test]
    fn test_validate_command_ids_duplicate() {
        let commands = vec![
            create_test_command_with_id("cmd1"),
            create_test_command_with_id("cmd2"),
            create_test_command_with_id("cmd1"), // Duplicate
        ];
        let result = validate_command_ids(&commands);
        assert!(matches!(result, Err(NonUniqueCommandId(_))));
    }

    #[test]
    fn test_validate_parameters_valid() {
        let cmd = create_test_command_with_param("name", "Hello {name}!");
        let params = cmd.parameters.as_ref().unwrap();
        assert!(validate_parameters(&cmd, params).is_ok());
    }

    #[test]
    fn test_validate_parameters_duplicate_ids() {
        let cmd = CommandDefinition {
            command: vec!["echo".to_string(), "{param}".to_string()],
            id: Some("test".to_string()),
            description: None,
            working_directory: None,
            parameters: Some(vec![
                ParameterDefinition {
                    id: "param".to_string(),
                    default: None,
                    description: None,
                },
                ParameterDefinition {
                    id: "param".to_string(), // Duplicate
                    default: None,
                    description: None,
                },
            ]),
            environment: None,
            metadata: None,
        };

        let params = cmd.parameters.as_ref().unwrap();
        let result = validate_parameters(&cmd, params);
        assert!(matches!(result, Err(NonUniqueParameterId(_, _))));
    }

    #[test]
    fn test_validate_parameters_not_found_in_template() {
        let cmd = create_test_command_with_param("missing", "Hello World!"); // No {missing} template
        let params = cmd.parameters.as_ref().unwrap();
        let result = validate_parameters(&cmd, params);
        assert!(matches!(result, Err(NotFoundParameterId(_, _))));
    }

    #[test]
    fn test_write_and_read_last_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let template = CommandExecutionTemplate {
            command: vec!["echo".to_string(), "test".to_string()],
            working_directory: Some("/tmp".to_string()),
            template_context: None,
            environment: Some({
                let mut env = std::collections::HashMap::new();
                env.insert("TEST".to_string(), "value".to_string());
                env
            }),
        };

        // Write the command
        assert!(write_last_command(temp_path, &template).is_ok());

        // Read it back
        let read_result = get_last_command(&temp_path.to_string()).unwrap();
        assert!(read_result.is_some());

        let read_template = read_result.unwrap();
        assert_eq!(read_template.command, template.command);
        assert_eq!(read_template.working_directory, template.working_directory);
        assert_eq!(read_template.environment, template.environment);
    }

    #[test]
    fn test_get_last_command_file_not_exists() {
        let nonexistent_path = "/this/path/does/not/exist.yml";
        let result = get_last_command(&nonexistent_path.to_string()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_command_definitions_valid_yaml() {
        let yaml_content = r#"
- id: "test_command"
  command: ["echo", "Hello World!"]
  description: "A test command"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml_content).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let result = get_command_definitions(&temp_path.to_string());
        assert!(result.is_ok());

        let commands = result.unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, Some("test_command".to_string()));
        assert_eq!(commands[0].command, vec!["echo", "Hello World!"]);
    }

    #[test]
    fn test_get_command_definitions_empty_file() {
        let yaml_content = "[]";

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml_content).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let result = get_command_definitions(&temp_path.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_command_definitions_invalid_yaml() {
        let yaml_content = "invalid: yaml: content: [";

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml_content).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let result = get_command_definitions(&temp_path.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_command_definitions_file_not_found() {
        let nonexistent_path = "/this/path/does/not/exist.yml";
        let result = get_command_definitions(&nonexistent_path.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_command_definitions_with_validation_errors() {
        let yaml_content = r#"
- id: "cmd1"
  command: ["echo", "test"]
- id: "cmd1"  # Duplicate ID
  command: ["echo", "test2"]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml_content).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let result = get_command_definitions(&temp_path.to_string());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NonUniqueCommandId(_)));
    }
}
