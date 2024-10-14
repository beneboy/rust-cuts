use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct ParameterDefinition {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CommandDefinition {
    pub command: Vec<String>,
    pub name: Option<String>,
    pub working_directory: Option<String>,
    pub parameters: Option<Vec<ParameterDefinition>>,
    pub environment: Option<HashMap<String, String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CommandExecutionTemplate {
    pub command: Vec<String>,
    pub working_directory: Option<String>,
    pub template_context: Option<HashMap<String, String>>,
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

impl Display for CommandDefinition {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        self.name.as_ref().map_or(Ok(()), |name| formatter.write_str(name))
    }
}

impl Display for CommandExecutionTemplate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter.write_str(self.command.join(" ").as_str())
    }
}
