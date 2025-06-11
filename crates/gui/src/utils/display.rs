use rust_cuts_core::command_definitions::CommandDefinition;

pub fn get_command_display_name(cmd: &CommandDefinition) -> String {
    match (&cmd.id, &cmd.description) {
        (Some(id), Some(desc)) => format!("{} ({})", id, desc),
        (Some(id), None) => id.clone(),
        (None, Some(desc)) => desc.clone(),
        (None, None) => cmd.command.join(" "),
    }
}