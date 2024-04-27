use derive_more::From;
use thiserror::Error;


pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{}", _0)]
    Custom(String),

    #[error("The sub process exiting with non-success code.")]
    SubProcessExitFailure,

    #[error("Error with sub process process: {}", _0)]
    SubProcessFailure(#[from] std::io::Error),

    #[error("Error parsing config file at `{}`: {}", .path, .original)]
    YamlFailure {action: String, file_description: String, path: String, original: serde_yaml::Error},

    #[error("No commands were found in the command definition YAML. Is `{}` empty?", .path)]
    EmptyCommandDefinition{path: String},

    #[error("IO error at path `{}`: {}", .path, .original)]
    IoError{path: String, original: std::io::Error},
}

impl Error {
    pub fn custom(val: impl std::fmt::Display) -> Self {
        Self::Custom(val.to_string())
    }

    pub fn yaml_error(action: String, file_description: String, path: String, original: serde_yaml::Error) -> Self {
        Self::YamlFailure{action, file_description, path, original}
    }

    pub fn io_error(path: String, original: std::io::Error) -> Self {
        Self::IoError{path, original}
    }
}


impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::custom(value)
    }
}
