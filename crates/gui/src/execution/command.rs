use rust_cuts_core::command_definitions::{CommandDefinition, TemplateParser};
use rust_cuts_core::interpolation::interpolate_command;
use std::collections::HashMap;
use std::process::Command;

pub async fn execute_command(
    cmd: CommandDefinition,
    params: HashMap<String, String>,
) -> Result<String, String> {
    // Use core's template parsing and interpolation
    let templates = match cmd.get_templates() {
        Ok(templates) => templates,
        Err(e) => return Err(format!("Template parse error: {}", e)),
    };

    let interpolated_command = match interpolate_command(&params, &templates) {
        Ok(cmd_parts) => cmd_parts,
        Err(e) => return Err(format!("Template interpolation error: {}", e)),
    };

    if interpolated_command.is_empty() {
        return Err("No command to execute".to_string());
    }

    // Build the command using the first part as the executable and rest as args
    let mut command = Command::new(&interpolated_command[0]);
    if interpolated_command.len() > 1 {
        command.args(&interpolated_command[1..]);
    }

    // Handle working directory with template interpolation
    if let Some(cwd) = &cmd.working_directory {
        let interpolated_cwd = interpolate_single_template(cwd, &params)?;
        command.current_dir(&interpolated_cwd);
    }

    // Handle environment variables with template interpolation
    if let Some(env_vars) = &cmd.environment {
        for (key, value) in env_vars {
            let interpolated_value = interpolate_single_template(value, &params)?;
            command.env(key, interpolated_value);
        }
    }

    // Execute and capture output
    match command.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        Err(e) => Err(format!("Failed to execute command: {}", e)),
    }
}

/// Helper function to interpolate a single template string
fn interpolate_single_template(
    template_str: &str,
    params: &HashMap<String, String>,
) -> Result<String, String> {
    use leon::Template;
    
    let template = match Template::parse(template_str) {
        Ok(t) => t,
        Err(e) => return Err(format!("Template parse error: {}", e)),
    };
    
    match template.render(params) {
        Ok(s) => Ok(s),
        Err(e) => return Err(format!("Template render error: {}", e)),
    }
}