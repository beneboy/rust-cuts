use crate::error::Result;
use indexmap::IndexSet;
use leon::{Item, Template};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CommandExecutionTemplate {
    pub command: Vec<String>,
    pub working_directory: Option<String>,
    pub template_context: Option<HashMap<String, ParameterDefinition>>,
    pub environment: Option<HashMap<String, String>>,
}

pub trait TemplateParser {
    fn get_command_templates(&self) -> &[String];

    fn get_templates(&self) -> Result<Vec<Template>> {
        let mut templates = Vec::new();

        for cmd in self.get_command_templates() {
            let template = Template::parse(cmd)?;
            templates.push(template);
        }
        Ok(templates)
    }

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
