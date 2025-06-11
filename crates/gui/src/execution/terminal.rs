use rust_cuts_core::command_definitions::{CommandDefinition, TemplateParser};
use rust_cuts_core::interpolation::interpolate_command;
use std::collections::HashMap;
use std::process::Command;

pub async fn run_in_terminal(
    cmd: CommandDefinition,
    params: HashMap<String, String>,
) -> Result<(), String> {
    // Use core's template parsing and interpolation for consistency
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

    // Build the full command string for the terminal
    let mut full_command = interpolated_command.join(" ");
    
    // Handle working directory by prefixing with 'cd' if needed
    if let Some(cwd) = &cmd.working_directory {
        let interpolated_cwd = interpolate_single_template(cwd, &params)?;
        full_command = format!("cd '{}' && {}", interpolated_cwd, full_command);
    }
    
    // Handle environment variables by prefixing with exports
    if let Some(env_vars) = &cmd.environment {
        let mut env_commands = Vec::new();
        for (key, value) in env_vars {
            let interpolated_value = interpolate_single_template(value, &params)?;
            env_commands.push(format!("export {}='{}'", key, interpolated_value));
        }
        
        if !env_commands.is_empty() {
            full_command = format!("{} && {}", env_commands.join(" && "), full_command);
        }
    }

    // Platform-specific terminal launching
    if cfg!(target_os = "macos") {
        launch_terminal_macos(&full_command)
    } else if cfg!(target_os = "linux") {
        launch_terminal_linux(&full_command)
    } else if cfg!(target_os = "windows") {
        launch_terminal_windows(&full_command)
    } else {
        Err("Unsupported operating system for terminal launching".to_string())
    }
}

fn launch_terminal_macos(full_command: &str) -> Result<(), String> {
    let terminal_command = format!(
        "tell application \"Terminal\" to activate\ntell application \"Terminal\" to do script \"{}\"",
        full_command.replace("\"", "\\\"")
    );

    let mut command = Command::new("osascript");
    command.arg("-e").arg(&terminal_command);

    match command.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Failed to launch terminal: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }
        Err(e) => Err(format!("Failed to execute osascript: {}", e)),
    }
}

fn launch_terminal_linux(full_command: &str) -> Result<(), String> {
    // Try common Linux terminal emulators in order of preference
    let terminals = [
        ("gnome-terminal", vec!["--", "bash", "-c"]),
        ("konsole", vec!["-e", "bash", "-c"]),
        ("xfce4-terminal", vec!["-e", "bash", "-c"]),
        ("mate-terminal", vec!["-e", "bash", "-c"]),
        ("lxterminal", vec!["-e", "bash", "-c"]),
        ("xterm", vec!["-e", "bash", "-c"]),
        ("urxvt", vec!["-e", "bash", "-c"]),
        ("alacritty", vec!["-e", "bash", "-c"]),
        ("kitty", vec!["bash", "-c"]),
        ("terminator", vec!["-e", "bash", "-c"]),
    ];

    for (terminal, args) in &terminals {
        // Check if terminal exists
        if Command::new("which").arg(terminal).output().is_ok() {
            let mut command = Command::new(terminal);
            command.args(args);
            command.arg(format!("{}; read -p 'Press Enter to close...'", full_command));
            
            match command.spawn() {
                Ok(_) => return Ok(()),
                Err(_) => continue, // Try next terminal
            }
        }
    }

    Err("No supported terminal emulator found on this Linux system".to_string())
}

fn launch_terminal_windows(full_command: &str) -> Result<(), String> {
    // Try Windows Terminal first (modern), then fall back to cmd
    
    // Try Windows Terminal (new Windows 10/11 terminal)
    if let Ok(_) = Command::new("wt").arg("--help").output() {
        let mut command = Command::new("wt");
        command.args(&["cmd", "/k", full_command]);
        
        match command.spawn() {
            Ok(_) => return Ok(()),
            Err(_) => {} // Fall through to next option
        }
    }

    // Try PowerShell
    if let Ok(_) = Command::new("powershell").arg("-Help").output() {
        let mut command = Command::new("powershell");
        command.args(&["-NoExit", "-Command", full_command]);
        
        match command.spawn() {
            Ok(_) => return Ok(()),
            Err(_) => {} // Fall through to next option
        }
    }

    // Fall back to cmd.exe
    let mut command = Command::new("cmd");
    command.args(&["/k", full_command]);
    
    match command.spawn() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to launch Windows terminal: {}", e)),
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