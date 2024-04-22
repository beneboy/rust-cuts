use std::fs::File;
use std::path::Path;
use crate::command_definitions::CommandDefinition;
use crate::command_definitions::LastCommandParameters;

fn get_reader(file_description: &str, path: &str) -> Result<File, String> {
    return match File::open(path) {
        Ok(reader) => Ok(reader),
        Err(e) => Err(format!("Could not read {} file at `{}`: {}", file_description, path, e))
    };
}

fn get_last_command_reader(last_command_path: &String) -> Result<Option<File>, String> {
    if !Path::exists(Path::new(last_command_path)) {
        return Ok(None);
    }

    return match get_reader("last command", last_command_path) {
        Ok(f) => Ok(Some(f)),
        Err(e) => Err(e),
    };
}

pub fn get_last_command(last_command_path: &String) -> Result<Option<LastCommandParameters>, String> {
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
            Err(format!("Error reading last command at at `{}`: {}", last_command_path, e).to_string())
        }
    }
}

pub fn write_last_command(path: &str, last_command: &LastCommandParameters) -> Result<(), String> {
    let f = File::create(path);

    let Ok( f) = f else {
        return Err(format!("Error opening last command path at `{}` for writing: {}", path, f.unwrap_err()));
    };

    return match serde_yaml::to_writer(f, &last_command) {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(format!("Error writing to last command path at `{}`: {}", path, e))
        }
    };
}

pub fn get_command_definitions(config_path: &String) -> Result<Vec<CommandDefinition>, String> {
    let config_reader_result = get_reader("config", &config_path);

    let parsing_result: serde_yaml::Result<Vec<CommandDefinition>>;

    {
        let Ok(ref config_reader) = config_reader_result else {
            return Err(format!("{}", config_reader_result.unwrap_err()))
        };

        parsing_result = serde_yaml::from_reader(config_reader);
    }

    let Ok(parsed_command_defs) = parsing_result else {
        return Err(format!(
            "Failed to parse config file at `{}`:\n{}",
            config_path, parsing_result.unwrap_err()
        ));
    };

    if parsed_command_defs.len() == 0 {
        return Err(format!("No commands were found in the command definition YAML. Is `{}` empty?", config_path));
    }
    Ok(parsed_command_defs)
}
