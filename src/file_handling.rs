use std::fs::File;
use std::path::Path;

use crate::command_definitions::{CommandDefinition, CommandExecutionTemplate};
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
        Err(Error::empty_command_definition(config_path.to_string()))
    } else {
        Ok(parsed_command_defs)
    }
}
