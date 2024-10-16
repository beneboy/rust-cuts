use crate::error::{Error, Result};
use crossterm::style::Color;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct ParameterDefinition {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ColorDefinition {
    rgb: Option<(u8, u8, u8)>,
    ansi: Option<u8>,
    name: Option<String>,
}

impl ColorDefinition {
    pub fn as_crossterm_color(&self) -> Result<Option<Color>> {
        let defined_count = [self.rgb.is_some(), self.ansi.is_some(), self.name.is_some()]
            .iter()
            .filter(|&&x| x)
            .count();

        // Error if more than one field is defined
        if defined_count > 1 {
            return Err(Error::MultipleColorTypes);
        }

        // Convert to crossterm Color
        Ok(match (self.rgb, self.ansi, &self.name) {
            (Some((r, g, b)), None, None) => Some(Color::Rgb { r, g, b }),
            (None, Some(ansi), None) => Some(Color::AnsiValue(ansi)),
            (None, None, Some(name)) => Some(match name.to_lowercase().as_str() {
                "black" => Color::Black,
                "darkgrey" => Color::DarkGrey,
                "red" => Color::Red,
                "darkred" => Color::DarkRed,
                "green" => Color::Green,
                "darkgreen" => Color::DarkGreen,
                "yellow" => Color::Yellow,
                "darkyellow" => Color::DarkYellow,
                "blue" => Color::Blue,
                "darkblue" => Color::DarkBlue,
                "magenta" => Color::Magenta,
                "darkmagenta" => Color::DarkMagenta,
                "cyan" => Color::Cyan,
                "darkcyan" => Color::DarkCyan,
                "white" => Color::White,
                "grey" => Color::Grey,
                _ => return Err(Error::UnknownColorName(name.to_string())),
            }),
            (None, None, None) => None,
            _ => unreachable!(), // This case is prevented by the earlier check
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CommandMetadata {
    pub foreground_color: Option<ColorDefinition>,
    pub background_color: Option<ColorDefinition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CommandDefinition {
    pub command: Vec<String>,
    pub name: Option<String>,
    pub working_directory: Option<String>,
    pub parameters: Option<Vec<ParameterDefinition>>,
    pub environment: Option<HashMap<String, String>>,
    pub metadata: Option<CommandMetadata>,
}

fn color_from_metadata_attribute(
    color_definition: &Option<ColorDefinition>,
) -> Result<Option<Color>> {
    match color_definition {
        None => Ok(None),
        Some(color_definition) => color_definition.as_crossterm_color(),
    }
}

impl CommandDefinition {
    pub fn foreground_color(&self) -> Result<Option<Color>> {
        if let Some(metadata) = &self.metadata {
            color_from_metadata_attribute(&metadata.foreground_color)
        } else {
            Ok(None)
        }
    }

    pub fn background_color(&self) -> Result<Option<Color>> {
        if let Some(metadata) = &self.metadata {
            color_from_metadata_attribute(&metadata.background_color)
        } else {
            Ok(None)
        }
    }
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
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.name
            .as_ref()
            .map_or(Ok(()), |name| formatter.write_str(name))
    }
}

impl Display for CommandExecutionTemplate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.command.join(" ").as_str())
    }
}
