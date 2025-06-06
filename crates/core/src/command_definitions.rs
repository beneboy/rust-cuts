use crate::error::Result;
use indexmap::IndexSet;
use leon::{Item, Template};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

/// Defines a parameter that can be used in command templates.
///
/// Parameters allow commands to be templated with user-provided values.
/// Each parameter has an identifier that corresponds to a template variable
/// in the command, and can optionally provide a default value and description.
///
/// # Examples
///
/// ```
/// use rust_cuts_core::command_definitions::ParameterDefinition;
///
/// let param = ParameterDefinition {
///     id: "host".to_string(),
///     default: Some("localhost".to_string()),
///     description: Some("Target hostname".to_string()),
/// };
/// ```
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct ParameterDefinition {
    /// Unique identifier for this parameter, must match template variables in command
    pub id: String,
    /// Default value to use if no value is provided by the user
    pub default: Option<String>,
    /// Human-readable description of what this parameter is for
    pub description: Option<String>,
}

impl Display for ParameterDefinition {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        // Always show the id
        write!(formatter, "`{}`", self.id)?;

        // Add description if present
        if let Some(desc) = &self.description {
            write!(formatter, " ({})", desc)?;
        }

        Ok(())
    }
}

/// Defines a color that can be used for command styling.
///
/// Colors can be specified using RGB values, ANSI codes, or named colors.
/// Only one color specification method should be used per ColorDefinition.
#[derive(Deserialize, Debug, Clone)]
pub struct ColorDefinition {
    /// RGB color specification as (red, green, blue) with values 0-255
    pub rgb: Option<(u8, u8, u8)>,
    /// ANSI color code (0-255)
    pub ansi: Option<u8>,
    /// Named color (e.g., "red", "blue", "green")
    pub name: Option<String>,
}

/// Metadata for customizing the appearance of commands in the UI.
///
/// Allows setting foreground and background colors for better visual
/// organization of commands in the command selection interface.
#[derive(Deserialize, Debug, Clone)]
pub struct CommandMetadata {
    /// Color for the command text
    pub foreground_color: Option<ColorDefinition>,
    /// Color for the command background
    pub background_color: Option<ColorDefinition>,
}

/// Defines a command that can be executed through rust-cuts.
///
/// A command definition includes the command to execute, optional parameters
/// for templating, working directory, environment variables, and metadata
/// for customization.
///
/// # Examples
///
/// ```
/// use rust_cuts_core::command_definitions::CommandDefinition;
/// use std::collections::HashMap;
///
/// let cmd = CommandDefinition {
///     command: vec!["echo".to_string(), "Hello {name}!".to_string()],
///     id: Some("greet".to_string()),
///     description: Some("Greet someone".to_string()),
///     working_directory: None,
///     parameters: None,
///     environment: None,
///     metadata: None,
/// };
/// ```
#[derive(Deserialize, Debug, Clone)]
pub struct CommandDefinition {
    /// The command and arguments to execute, may contain template variables like {param}
    pub command: Vec<String>,
    /// Optional unique identifier for this command
    pub id: Option<String>,
    /// Human-readable description of what this command does
    pub description: Option<String>,
    /// Directory to change to before executing the command
    pub working_directory: Option<String>,
    /// Parameters that can be used to template the command
    pub parameters: Option<Vec<ParameterDefinition>>,
    /// Environment variables to set when executing the command
    pub environment: Option<HashMap<String, String>>,
    /// Styling metadata for the command UI
    pub metadata: Option<CommandMetadata>,
}

/// A command template ready for execution with resolved parameters.
///
/// This represents a command definition that has been processed and is ready
/// to be executed, potentially with parameter values filled in.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CommandExecutionTemplate {
    /// The command and arguments to execute
    pub command: Vec<String>,
    /// Directory to change to before executing the command
    pub working_directory: Option<String>,
    /// Context containing parameter definitions for templating
    pub template_context: Option<HashMap<String, ParameterDefinition>>,
    /// Environment variables to set when executing the command
    pub environment: Option<HashMap<String, String>>,
}

/// Trait for objects that can parse template variables from command strings.
///
/// This trait provides functionality to extract template variables (like {param})
/// from command definitions and create template objects for parameter substitution.
pub trait TemplateParser {
    /// Returns the command strings that may contain template variables.
    fn get_command_templates(&self) -> &[String];

    /// Parses the command strings into Template objects for variable substitution.
    ///
    /// # Errors
    ///
    /// Returns an error if any command string contains invalid template syntax.
    fn get_templates(&self) -> Result<Vec<Template>> {
        let mut templates = Vec::new();

        for cmd in self.get_command_templates() {
            let template = Template::parse(cmd)?;
            templates.push(template);
        }
        Ok(templates)
    }

    /// Extracts all template variables from the command strings in order of first appearance.
    ///
    /// Returns an ordered set of variable names that need to be provided for
    /// template substitution. Variables are ordered by their first occurrence
    /// in the command templates.
    ///
    /// # Errors
    ///
    /// Returns an error if template parsing fails.
    fn get_ordered_context_variables(&self) -> Result<IndexSet<String>> {
        let mut variables: IndexSet<String> = IndexSet::new();
        for template in self.get_templates()?.iter() {
            for item in template.items.iter() {
                match item {
                    Item::Text(_) => {
                        // normal text, do nothing
                    }
                    Item::Key(k) => {
                        // IndexSet keeps order, but won't insert if the value exists
                        variables.insert(k.to_string());
                    }
                }
            }
        }
        Ok(variables)
    }
}

impl TemplateParser for CommandDefinition {
    fn get_command_templates(&self) -> &[String] {
        &self.command
    }
}

impl TemplateParser for CommandExecutionTemplate {
    fn get_command_templates(&self) -> &[String] {
        &self.command
    }
}

impl Display for CommandDefinition {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match (&self.id, &self.description) {
            (Some(id), Some(desc)) => {
                // Both id and description exist
                write!(formatter, "{} ({})", id, desc)
            }
            (Some(id), None) => {
                // Only id exists
                formatter.write_str(id)
            }
            (None, Some(desc)) => {
                // Only description exists
                formatter.write_str(desc)
            }
            (None, None) => {
                // Neither exists, fall back to the command itself
                write!(formatter, "{}", self.command.join(" "))
            }
        }
    }
}

impl CommandExecutionTemplate {
    /// Creates a new CommandExecutionTemplate from a CommandDefinition.
    ///
    /// This copies the essential execution information from a command definition
    /// while leaving the template_context empty for later parameter resolution.
    ///
    /// # Arguments
    ///
    /// * `value` - The command definition to convert
    pub fn from_command_definition(value: &CommandDefinition) -> Self {
        Self {
            command: value.command.clone(),
            working_directory: value.working_directory.clone(),
            template_context: None,
            environment: value.environment.clone(),
        }
    }
}

impl Display for CommandExecutionTemplate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.command.join(" ").as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_parameter() -> ParameterDefinition {
        ParameterDefinition {
            id: "test_param".to_string(),
            default: Some("default_value".to_string()),
            description: Some("Test parameter".to_string()),
        }
    }

    fn create_test_command() -> CommandDefinition {
        CommandDefinition {
            command: vec!["echo".to_string(), "Hello {name}!".to_string()],
            id: Some("test_command".to_string()),
            description: Some("Test command".to_string()),
            working_directory: Some("/tmp".to_string()),
            parameters: Some(vec![ParameterDefinition {
                id: "name".to_string(),
                default: Some("World".to_string()),
                description: Some("Name to greet".to_string()),
            }]),
            environment: Some({
                let mut env = HashMap::new();
                env.insert("TEST_VAR".to_string(), "test_value".to_string());
                env
            }),
            metadata: None,
        }
    }

    #[test]
    fn test_parameter_definition_display() {
        let param = create_test_parameter();
        let display_str = format!("{}", param);
        assert_eq!(display_str, "`test_param` (Test parameter)");
    }

    #[test]
    fn test_parameter_definition_display_no_description() {
        let param = ParameterDefinition {
            id: "no_desc".to_string(),
            default: None,
            description: None,
        };
        let display_str = format!("{}", param);
        assert_eq!(display_str, "`no_desc`");
    }

    #[test]
    fn test_command_definition_display_with_id_and_description() {
        let cmd = create_test_command();
        let display_str = format!("{}", cmd);
        assert_eq!(display_str, "test_command (Test command)");
    }

    #[test]
    fn test_command_definition_display_id_only() {
        let mut cmd = create_test_command();
        cmd.description = None;
        let display_str = format!("{}", cmd);
        assert_eq!(display_str, "test_command");
    }

    #[test]
    fn test_command_definition_display_description_only() {
        let mut cmd = create_test_command();
        cmd.id = None;
        let display_str = format!("{}", cmd);
        assert_eq!(display_str, "Test command");
    }

    #[test]
    fn test_command_definition_display_fallback_to_command() {
        let mut cmd = create_test_command();
        cmd.id = None;
        cmd.description = None;
        let display_str = format!("{}", cmd);
        assert_eq!(display_str, "echo Hello {name}!");
    }

    #[test]
    fn test_command_execution_template_from_command_definition() {
        let cmd = create_test_command();
        let template = CommandExecutionTemplate::from_command_definition(&cmd);

        assert_eq!(template.command, cmd.command);
        assert_eq!(template.working_directory, cmd.working_directory);
        assert_eq!(template.environment, cmd.environment);
        assert!(template.template_context.is_none());
    }

    #[test]
    fn test_command_execution_template_display() {
        let template = CommandExecutionTemplate {
            command: vec!["ls".to_string(), "-la".to_string()],
            working_directory: None,
            template_context: None,
            environment: None,
        };
        let display_str = format!("{}", template);
        assert_eq!(display_str, "ls -la");
    }

    #[test]
    fn test_template_parser_get_ordered_context_variables() {
        let cmd = CommandDefinition {
            command: vec![
                "echo".to_string(),
                "Hello {name}!".to_string(),
                "You are {age} years old.".to_string(),
                "Your name is {name} again.".to_string(),
            ],
            id: None,
            description: None,
            working_directory: None,
            parameters: None,
            environment: None,
            metadata: None,
        };

        let variables = cmd.get_ordered_context_variables().unwrap();
        assert_eq!(variables.len(), 2);
        assert_eq!(variables[0], "name");
        assert_eq!(variables[1], "age");
    }

    #[test]
    fn test_template_parser_no_variables() {
        let cmd = CommandDefinition {
            command: vec!["echo".to_string(), "Hello World!".to_string()],
            id: None,
            description: None,
            working_directory: None,
            parameters: None,
            environment: None,
            metadata: None,
        };

        let variables = cmd.get_ordered_context_variables().unwrap();
        assert!(variables.is_empty());
    }

    #[test]
    fn test_template_parser_complex_variables() {
        let cmd = CommandDefinition {
            command: vec![
                "ssh".to_string(),
                "-i".to_string(),
                "{key_path}".to_string(),
                "{user}@{host}".to_string(),
            ],
            id: None,
            description: None,
            working_directory: None,
            parameters: None,
            environment: None,
            metadata: None,
        };

        let variables = cmd.get_ordered_context_variables().unwrap();
        assert_eq!(variables.len(), 3);
        assert_eq!(variables[0], "key_path");
        assert_eq!(variables[1], "user");
        assert_eq!(variables[2], "host");
    }

    #[test]
    fn test_color_definition_variants() {
        // Test RGB color
        let rgb_color = ColorDefinition {
            rgb: Some((255, 0, 0)),
            ansi: None,
            name: None,
        };
        assert_eq!(rgb_color.rgb, Some((255, 0, 0)));

        // Test ANSI color
        let ansi_color = ColorDefinition {
            rgb: None,
            ansi: Some(9),
            name: None,
        };
        assert_eq!(ansi_color.ansi, Some(9));

        // Test named color
        let named_color = ColorDefinition {
            rgb: None,
            ansi: None,
            name: Some("red".to_string()),
        };
        assert_eq!(named_color.name, Some("red".to_string()));
    }
}
