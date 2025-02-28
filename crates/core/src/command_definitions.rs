use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ParameterDefinition {
    pub id: String,
    pub default: Option<String>,
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

#[derive(Deserialize, Debug, Clone)]
pub struct ColorDefinition {
    pub rgb: Option<(u8, u8, u8)>,
    pub ansi: Option<u8>,
    pub name: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CommandMetadata {
    pub foreground_color: Option<ColorDefinition>,
    pub background_color: Option<ColorDefinition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CommandDefinition {
    pub command: Vec<String>,
    pub id: Option<String>,
    pub description: Option<String>,
    pub working_directory: Option<String>,
    pub parameters: Option<Vec<ParameterDefinition>>,
    pub environment: Option<HashMap<String, String>>,
    pub metadata: Option<CommandMetadata>,
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


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CommandExecutionTemplate {
    pub command: Vec<String>,
    pub working_directory: Option<String>,
    pub template_context: Option<HashMap<String, ParameterDefinition>>,
    pub environment: Option<HashMap<String, String>>,
}

impl CommandExecutionTemplate {
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
