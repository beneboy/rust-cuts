use std::fs::File;
use std::path::Path;
use crate::command_definitions::CommandDefinition;
use crate::command_definitions::LastCommandParameters;
use crate::rc::error::{Result, Error };


fn get_reader(file_description: &str, path: &str) -> Result<File> {
    return match File::open(path) {
        Ok(reader) => Ok(reader),
        Err(e) => Err(format!("Could not read {} file at `{}`: {}", file_description, path, e).as_str().into())
    };
}

fn get_last_command_reader(last_command_path: &String) -> Result<Option<File>> {
    if !Path::exists(Path::new(last_command_path)) {
        return Ok(None);
    }

    return match get_reader("last command", last_command_path) {
        Ok(f) => Ok(Some(f)),
        Err(e) => Err(e),
    };
}

pub fn get_last_command(last_command_path: &String) -> Result<Option<LastCommandParameters>> {
    let last_command_reader = get_last_command_reader(last_command_path)?;
    let Some(last_command_reader) = last_command_reader else {
        return Ok(None);
    };

    let last_command_parameter: serde_yaml::Result<LastCommandParameters> = serde_yaml::from_reader(last_command_reader);

    match last_command_parameter {
        Ok(last_command_parameter) => {
            Ok(Some(last_command_parameter))
        }
        Err(e) => {
            Err(format!("Error reading last command at at `{}`: {}", last_command_path, e).as_str().into())
        }
    }
}

pub fn write_last_command(path: &str, last_command: &LastCommandParameters) -> Result<()> {
    let f = File::create(path);

    let Ok(f) = f else {
        return Err(Error::io_error(path.to_string(), f.unwrap_err()))
    };

    return match serde_yaml::to_writer(f, &last_command) {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(Error::yaml_error(path.to_string(), "writing".to_string(), "last command".to_string(), e))
        }
    };
}

pub fn get_command_definitions(config_path: &String) -> Result<Vec<CommandDefinition>> {
    let config_reader_result = get_reader("config", &config_path);

    let parsing_result: serde_yaml::Result<Vec<CommandDefinition>>;

    {
        let Ok(ref config_reader) = config_reader_result else {
            return Err(format!("{}", config_reader_result.unwrap_err()).as_str().into());
        };

        parsing_result = serde_yaml::from_reader(config_reader);
    }

    let Ok(parsed_command_defs) = parsing_result else {
        return Err(
            Error::yaml_error(config_path.clone(), "reading".to_string(), "config".to_string(), parsing_result.unwrap_err()).into()
        );
    };

    if parsed_command_defs.len() == 0 {
        return Err(Error::EmptyCommandDefinition { path: config_path.to_string() });
    }
    Ok(parsed_command_defs)
}
