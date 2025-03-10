use rust_cuts_core::command_definitions::{ColorDefinition, CommandDefinition};
use crossterm::style::Color;
use rust_cuts_core::error::{Error, Result};

/// Trait for converting color definitions to terminal colors
pub trait AsTermColor {
    fn as_crossterm_color(&self) -> Result<Option<Color>>;
}

impl AsTermColor for ColorDefinition {
    fn as_crossterm_color(&self) -> Result<Option<Color>> {
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

/// Helper function to extract color from metadata
pub fn color_from_metadata_attribute(
    color_definition: &Option<ColorDefinition>,
) -> Result<Option<Color>> {
    match color_definition {
        None => Ok(None),
        Some(color_definition) => color_definition.as_crossterm_color(),
    }
}

/// Trait for accessing CommandDefinition colors
pub trait CommandDefinitionColor {
    fn foreground_color(&self) -> Result<Option<Color>>;
    fn background_color(&self) -> Result<Option<Color>>;
}

impl CommandDefinitionColor for CommandDefinition {
    fn foreground_color(&self) -> Result<Option<Color>> {
        if let Some(metadata) = &self.metadata {
            color_from_metadata_attribute(&metadata.foreground_color)
        } else {
            Ok(None)
        }
    }

    fn background_color(&self) -> Result<Option<Color>> {
        if let Some(metadata) = &self.metadata {
            color_from_metadata_attribute(&metadata.background_color)
        } else {
            Ok(None)
        }
    }
}