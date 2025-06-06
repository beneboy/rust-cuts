//! Integration tests for rust-cuts-core
//!
//! These tests verify that the core functionality works together correctly
//! by testing complete workflows end-to-end.

use rust_cuts_core::{
    command_definitions::{
        CommandDefinition, CommandExecutionTemplate, ParameterDefinition, TemplateParser,
    },
    config::{expand_working_directory, get_config_path, get_last_command_path},
    file_handling::{get_command_definitions, get_last_command, write_last_command},
};
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

/// Test loading and parsing a complete command configuration
#[test]
fn test_complete_command_configuration_workflow() {
    let yaml_content = r#"
- id: "greet"
  command: ["echo", "Hello {name}!"]
  description: "Greet someone by name"
  working_directory: "~/projects"
  parameters:
    - id: "name"
      default: "World"
      description: "Name to greet"
  environment:
    GREETING_LANG: "en"
  metadata:
    foreground_color:
      name: "green"
    background_color:
      ansi: 240

- id: "deploy"
  command: ["kubectl", "apply", "-f", "{manifest}"]
  description: "Deploy Kubernetes manifest"
  parameters:
    - id: "manifest"
      description: "Path to manifest file"
  environment:
    KUBECONFIG: "/path/to/kubeconfig"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{yaml_content}").unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    // Load the command definitions
    let commands = get_command_definitions(&temp_path).unwrap();

    assert_eq!(commands.len(), 2);

    // Test first command (greet)
    let greet_cmd = &commands[0];
    assert_eq!(greet_cmd.id, Some("greet".to_string()));
    assert_eq!(greet_cmd.command, vec!["echo", "Hello {name}!"]);
    assert_eq!(
        greet_cmd.description,
        Some("Greet someone by name".to_string())
    );
    assert_eq!(greet_cmd.working_directory, Some("~/projects".to_string()));

    let params = greet_cmd.parameters.as_ref().unwrap();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].id, "name");
    assert_eq!(params[0].default, Some("World".to_string()));

    let env = greet_cmd.environment.as_ref().unwrap();
    assert_eq!(env.get("GREETING_LANG"), Some(&"en".to_string()));

    // Test metadata
    let metadata = greet_cmd.metadata.as_ref().unwrap();
    assert!(metadata.foreground_color.is_some());
    assert!(metadata.background_color.is_some());

    // Test second command (deploy)
    let deploy_cmd = &commands[1];
    assert_eq!(deploy_cmd.id, Some("deploy".to_string()));
    assert_eq!(
        deploy_cmd.command,
        vec!["kubectl", "apply", "-f", "{manifest}"]
    );

    let deploy_params = deploy_cmd.parameters.as_ref().unwrap();
    assert_eq!(deploy_params.len(), 1);
    assert_eq!(deploy_params[0].id, "manifest");
    assert!(deploy_params[0].default.is_none());
}

/// Test template variable extraction from complex commands
#[test]
fn test_template_variable_extraction_workflow() {
    let yaml_content = r#"
- id: "complex_ssh"
  command: ["ssh", "-i", "{key_path}", "-p", "{port}", "{user}@{host}", "{remote_command}"]
  parameters:
    - id: "key_path"
      default: "~/.ssh/id_rsa"
    - id: "port"
      default: "22"
    - id: "user"
      default: "ubuntu"
    - id: "host"
    - id: "remote_command"
      default: "ls -la"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{yaml_content}").unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    let commands = get_command_definitions(&temp_path).unwrap();
    let cmd = &commands[0];

    // Test template variable extraction
    let variables = cmd.get_ordered_context_variables().unwrap();
    assert_eq!(variables.len(), 5);
    assert_eq!(variables[0], "key_path");
    assert_eq!(variables[1], "port");
    assert_eq!(variables[2], "user");
    assert_eq!(variables[3], "host");
    assert_eq!(variables[4], "remote_command");

    // Verify all parameters have corresponding template variables
    let params = cmd.parameters.as_ref().unwrap();
    for param in params {
        assert!(variables.contains(&param.id));
    }
}

/// Test last command persistence workflow
#[test]
fn test_last_command_persistence_workflow() {
    // Use a path that doesn't exist initially
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path().join("last_command.yml");
    let temp_path_str = temp_path.to_str().unwrap().to_string();

    // Initially, no last command should exist
    let initial_result = get_last_command(&temp_path_str).unwrap();
    assert!(initial_result.is_none());

    // Create and save a command execution template
    let mut env = HashMap::new();
    env.insert("NODE_ENV".to_string(), "production".to_string());
    env.insert("PORT".to_string(), "3000".to_string());

    let mut context = HashMap::new();
    context.insert(
        "app_name".to_string(),
        ParameterDefinition {
            id: "app_name".to_string(),
            default: Some("my-app".to_string()),
            description: Some("Application name".to_string()),
        },
    );

    let template = CommandExecutionTemplate {
        command: vec![
            "npm".to_string(),
            "start".to_string(),
            "--".to_string(),
            "{app_name}".to_string(),
        ],
        working_directory: Some("/home/user/projects".to_string()),
        template_context: Some(context),
        environment: Some(env),
    };

    // Write the last command
    write_last_command(&temp_path_str, &template).unwrap();

    // Read it back and verify
    let saved_result = get_last_command(&temp_path_str).unwrap();
    assert!(saved_result.is_some());

    let saved_template = saved_result.unwrap();
    assert_eq!(saved_template.command, template.command);
    assert_eq!(saved_template.working_directory, template.working_directory);
    assert_eq!(saved_template.environment, template.environment);
    assert_eq!(saved_template.template_context, template.template_context);
}

/// Test configuration path resolution workflow
#[test]
fn test_configuration_path_workflow() {
    // Test default paths
    let default_config = get_config_path(&None);
    assert!(default_config.contains("commands.yml"));
    assert!(!default_config.starts_with('~')); // Should be expanded

    let default_last_cmd = get_last_command_path(&None);
    assert!(default_last_cmd.contains("last_command.yml"));
    assert!(!default_last_cmd.starts_with('~')); // Should be expanded

    // Test custom paths
    let custom_config = get_config_path(&Some("/custom/config.yml".to_string()));
    assert_eq!(custom_config, "/custom/config.yml");

    let custom_last_cmd = get_last_command_path(&Some("/custom/last.yml".to_string()));
    assert_eq!(custom_last_cmd, "/custom/last.yml");

    // Test working directory expansion
    let expanded_wd = expand_working_directory(&Some("~/dev/project".to_string()));
    assert!(expanded_wd.is_some());
    let expanded = expanded_wd.unwrap();
    assert!(!expanded.starts_with('~'));
    assert!(expanded.contains("dev/project"));

    let none_wd = expand_working_directory(&None);
    assert!(none_wd.is_none());
}

/// Test command definition to execution template conversion
#[test]
fn test_command_definition_to_execution_template_workflow() {
    let mut env = HashMap::new();
    env.insert("TEST_ENV".to_string(), "test_value".to_string());

    let params = vec![ParameterDefinition {
        id: "target".to_string(),
        default: Some("localhost".to_string()),
        description: Some("Target host".to_string()),
    }];

    let cmd_def = CommandDefinition {
        command: vec![
            "ping".to_string(),
            "-c".to_string(),
            "3".to_string(),
            "{target}".to_string(),
        ],
        id: Some("ping_test".to_string()),
        description: Some("Ping a target host".to_string()),
        working_directory: Some("/tmp".to_string()),
        parameters: Some(params),
        environment: Some(env.clone()),
        metadata: None,
    };

    // Convert to execution template
    let exec_template = CommandExecutionTemplate::from_command_definition(&cmd_def);

    assert_eq!(exec_template.command, cmd_def.command);
    assert_eq!(exec_template.working_directory, cmd_def.working_directory);
    assert_eq!(exec_template.environment, Some(env));
    assert!(exec_template.template_context.is_none()); // Should be empty initially
}

/// Test error handling for invalid configurations
#[test]
fn test_error_handling_workflow() {
    // Test empty configuration
    let empty_yaml = "[]";
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{empty_yaml}").unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    let result = get_command_definitions(&temp_path);
    assert!(result.is_err());

    // Test invalid YAML syntax
    let invalid_yaml = "invalid: yaml: [structure";
    let mut temp_file2 = NamedTempFile::new().unwrap();
    write!(temp_file2, "{invalid_yaml}").unwrap();
    let temp_path2 = temp_file2.path().to_str().unwrap().to_string();

    let result2 = get_command_definitions(&temp_path2);
    assert!(result2.is_err());

    // Test duplicate command IDs
    let duplicate_ids_yaml = r#"
- id: "duplicate"
  command: ["echo", "first"]
- id: "duplicate"
  command: ["echo", "second"]
"#;
    let mut temp_file3 = NamedTempFile::new().unwrap();
    write!(temp_file3, "{duplicate_ids_yaml}").unwrap();
    let temp_path3 = temp_file3.path().to_str().unwrap().to_string();

    let result3 = get_command_definitions(&temp_path3);
    assert!(result3.is_err());

    // Test parameter ID not found in template
    let missing_param_yaml = r#"
- id: "test"
  command: ["echo", "Hello World"]
  parameters:
    - id: "missing_param"
"#;
    let mut temp_file4 = NamedTempFile::new().unwrap();
    write!(temp_file4, "{missing_param_yaml}").unwrap();
    let temp_path4 = temp_file4.path().to_str().unwrap().to_string();

    let result4 = get_command_definitions(&temp_path4);
    assert!(result4.is_err());
}

/// Test display formatting for various command configurations
#[test]
fn test_display_formatting_workflow() {
    // Test command with ID and description
    let cmd1 = CommandDefinition {
        command: vec!["ls".to_string(), "-la".to_string()],
        id: Some("list_files".to_string()),
        description: Some("List all files".to_string()),
        working_directory: None,
        parameters: None,
        environment: None,
        metadata: None,
    };
    assert_eq!(format!("{cmd1}"), "list_files (List all files)");

    // Test command with only ID
    let cmd2 = CommandDefinition {
        command: vec!["pwd".to_string()],
        id: Some("show_dir".to_string()),
        description: None,
        working_directory: None,
        parameters: None,
        environment: None,
        metadata: None,
    };
    assert_eq!(format!("{cmd2}"), "show_dir");

    // Test command with only description
    let cmd3 = CommandDefinition {
        command: vec!["date".to_string()],
        id: None,
        description: Some("Show current date".to_string()),
        working_directory: None,
        parameters: None,
        environment: None,
        metadata: None,
    };
    assert_eq!(format!("{cmd3}"), "Show current date");

    // Test command with neither ID nor description (fallback to command)
    let cmd4 = CommandDefinition {
        command: vec!["whoami".to_string()],
        id: None,
        description: None,
        working_directory: None,
        parameters: None,
        environment: None,
        metadata: None,
    };
    assert_eq!(format!("{cmd4}"), "whoami");

    // Test execution template display
    let exec_template = CommandExecutionTemplate {
        command: vec!["git".to_string(), "status".to_string()],
        working_directory: None,
        template_context: None,
        environment: None,
    };
    assert_eq!(format!("{exec_template}"), "git status");
}
