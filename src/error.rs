use leon::{ParseError, RenderError};
use log::error;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("The sub process exiting with non-success code.")]
    SubProcessExit,

    #[error("Error with sub process process: {}", _0)]
    SubProcess(#[from] std::io::Error),

    #[error("Error {} {} file at `{}`: {}", .action, .file_description, .path, .original)]
    Yaml {
        action: String,
        file_description: String,
        path: String,
        original: serde_yaml::Error,
    },

    #[error("For a color, only one of `rgb`, `ansi` or `name` should be defined.")]
    MultipleColorTypes,

    #[error("Unknown color name: \"{}\"", _0)]
    UnknownColorName(String),

    #[error("No commands were found in the command definition YAML. Is `{}` empty?", .path)]
    EmptyCommandDefinition { path: String },

    #[error("IO error with {} file at path `{}`: {}", .file_description, .path, .original)]
    Io {
        file_description: String,
        path: String,
        original: std::io::Error,
    },

    #[error("Error parsing placeholder string: {}", .0)]
    Parse(#[from] ParseError),

    #[error("Error placeholder template string: {}", .0)]
    Render(#[from] RenderError),

    #[error("Rerun flag specified with an index is invalid.")]
    RerunWithIndex,

    #[error("Misc error: {}", .0)]
    Misc(String),

    #[error("STDIO error: {}", .0)]
    Stdio(std::io::Error),
}

impl Error {
    pub fn empty_command_definition(path: String) -> Self {
        Self::EmptyCommandDefinition { path }
    }

    pub fn yaml_error(
        action: String,
        file_description: String,
        path: String,
        original: serde_yaml::Error,
    ) -> Self {
        Self::Yaml {
            action,
            file_description,
            path,
            original,
        }
    }

    pub fn io_error(file_description: String, path: String, original: std::io::Error) -> Self {
        Self::Io {
            file_description,
            path,
            original,
        }
    }
}
