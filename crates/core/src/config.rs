const DEFAULT_CONFIG_PATH: &str = "~/.rust-cuts/commands.yml";
const DEFAULT_LAST_COMMAND_PATH: &str = "~/.rust-cuts/last_command.yml";

pub const DEFAULT_SHELL: &str = "/bin/bash";

pub fn get_config_path(config_path_arg: &Option<String>) -> String {
    let config_path = match config_path_arg {
        Some(last_command_path) => last_command_path,
        None => DEFAULT_CONFIG_PATH,
    };

    shellexpand::tilde(config_path).to_string()
}

pub fn get_last_command_path(last_command_path_arg: &Option<String>) -> String {
    let last_command_path = match last_command_path_arg {
        Some(last_command_path) => last_command_path,
        None => DEFAULT_LAST_COMMAND_PATH,
    };

    shellexpand::tilde(last_command_path).to_string()
}

pub fn expand_working_directory(working_directory: &Option<String>) -> Option<String> {
    if let Some(working_directory) = working_directory {
        return Some({
            let expanded = shellexpand::tilde(working_directory);
            expanded.to_string()
        });
    }

    None
}