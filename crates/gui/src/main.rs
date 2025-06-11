use iced::widget::{button, container, row, scrollable, text, text_input, Column};
use iced::{Center, Element, Length, Task};
use rust_cuts_core::command_definitions::{CommandDefinition, ParameterDefinition, TemplateParser};
use rust_cuts_core::{config, file_handling};
use leon::Template;
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum Message {
    CommandSelected(usize),
    ParameterChanged(String, String),
    RunCommand,
    RunInTerminal,
    CommandExecuted(Result<String, String>),
    TerminalLaunched(Result<(), String>),
}

struct RustCuts {
    command_definitions: Vec<CommandDefinition>,
    selected_command: Option<usize>,
    parameter_values: HashMap<String, String>,
    output: Option<Result<String, String>>,
}

impl RustCuts {
    fn title(&self) -> String {
        "RustCuts GUI".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CommandSelected(index) => {
                self.selected_command = Some(index);
                self.parameter_values.clear();
                self.output = None;
                if let Some(cmd) = self.command_definitions.get(index) {
                    // Extract all template variables from command strings
                    if let Ok(template_vars) = cmd.get_ordered_context_variables() {
                        // Create parameter definitions, merging explicit params with template vars
                        let mut param_lookup: HashMap<String, ParameterDefinition> = HashMap::new();
                        
                        // First, add explicitly defined parameters
                        if let Some(params) = &cmd.parameters {
                            for param in params {
                                param_lookup.insert(param.id.clone(), param.clone());
                            }
                        }
                        
                        // Then, ensure all template variables have parameter definitions
                        for var_name in template_vars {
                            if !param_lookup.contains_key(&var_name) {
                                // Create a basic parameter definition for undefined template vars
                                param_lookup.insert(var_name.clone(), ParameterDefinition {
                                    id: var_name.clone(),
                                    default: None,
                                    description: None,
                                });
                            }
                            
                            // Set default values in parameter_values
                            let default_value = param_lookup.get(&var_name)
                                .and_then(|p| p.default.clone())
                                .unwrap_or_default();
                            self.parameter_values.insert(var_name, default_value);
                        }
                    }
                }
                Task::none()
            }
            Message::ParameterChanged(key, value) => {
                self.parameter_values.insert(key, value);
                Task::none()
            }
            Message::RunCommand => {
                if let Some(index) = self.selected_command {
                    if let Some(cmd) = self.command_definitions.get(index).cloned() {
                        let params = self.parameter_values.clone();
                        return Task::perform(
                            async move { execute_command(cmd, params).await },
                            Message::CommandExecuted,
                        );
                    }
                }
                Task::none()
            }
            Message::RunInTerminal => {
                if let Some(index) = self.selected_command {
                    if let Some(cmd) = self.command_definitions.get(index).cloned() {
                        let params = self.parameter_values.clone();
                        return Task::perform(
                            async move { run_in_terminal(cmd, params).await },
                            Message::TerminalLaunched,
                        );
                    }
                }
                Task::none()
            }
            Message::CommandExecuted(result) => {
                self.output = Some(result);
                Task::none()
            }
            Message::TerminalLaunched(result) => {
                // Only show output if there was an error
                if let Err(e) = result {
                    self.output = Some(Err(format!("Failed to launch terminal: {}", e)));
                }
                // Don't show any output for successful terminal launches
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let left_column = {
            let mut column = Column::new().spacing(5).padding(10);
            
            for (index, cmd) in self.command_definitions.iter().enumerate() {
                let is_selected = self.selected_command == Some(index);
                let name = get_command_display_name(cmd);
                let btn = button(text(name).size(16))
                    .width(Length::Fill)
                    .padding(10)
                    .style(if is_selected {
                        button::primary
                    } else {
                        button::secondary
                    })
                    .on_press(Message::CommandSelected(index));
                column = column.push(btn);
            }
            
            scrollable(column).height(Length::Fill)
        };

        let right_column = if let Some(index) = self.selected_command {
            if let Some(cmd) = self.command_definitions.get(index) {
                let mut details = Column::new()
                    .spacing(15)
                    .padding(20)
                    .push(text(get_command_display_name(cmd)).size(24))
                    .push(text(cmd.command.join(" ")).size(14));

                // Show parameter fields for all template variables
                if let Ok(template_vars) = cmd.get_ordered_context_variables() {
                    if !template_vars.is_empty() {
                        details = details.push(text("Parameters:").size(18));
                        
                        // Create parameter lookup from explicit definitions
                        let mut param_lookup: HashMap<String, ParameterDefinition> = HashMap::new();
                        if let Some(params) = &cmd.parameters {
                            for param in params {
                                param_lookup.insert(param.id.clone(), param.clone());
                            }
                        }
                        
                        for var_name in template_vars {
                            let value = self
                                .parameter_values
                                .get(&var_name)
                                .cloned()
                                .unwrap_or_default();
                            
                            let input = text_input(&format!("Enter {}", var_name), &value)
                                .on_input({
                                    let key = var_name.clone();
                                    move |val| Message::ParameterChanged(key.clone(), val)
                                })
                                .padding(8)
                                .size(16);
                            
                            let mut param_column = Column::new()
                                .spacing(5)
                                .push(text(var_name.clone()).size(14))
                                .push(input);
                            
                            // Add description if available from explicit parameter definition
                            if let Some(param_def) = param_lookup.get(&var_name) {
                                if let Some(desc) = &param_def.description {
                                    param_column = param_column.push(text(desc.clone()).size(12));
                                }
                            }
                            
                            details = details.push(param_column);
                        }
                    }
                }

                let button_row = row![
                    button(text("Run").size(16))
                        .padding([10, 20])
                        .style(button::success)
                        .on_press(Message::RunCommand),
                    button(text("Run In Terminal").size(16))
                        .padding([10, 20])
                        .style(button::primary)
                        .on_press(Message::RunInTerminal),
                ]
                .spacing(10);
                
                details = details.push(button_row);

                if let Some(output) = &self.output {
                    let output_text = match output {
                        Ok(stdout) => text(stdout).size(14),
                        Err(error) => text(error).size(14).color([0.8, 0.2, 0.2]),
                    };
                    details = details.push(
                        container(scrollable(output_text))
                            .padding(10)
                            .style(container::bordered_box)
                            .height(Length::FillPortion(2)),
                    );
                }

                details
            } else {
                Column::new()
            }
        } else {
            Column::new()
                .push(text("Select a command from the list").size(18))
                .align_x(Center)
        };

        let content = row![
            container(left_column)
                .width(Length::Fixed(250.0))
                .height(Length::Fill)
                .style(container::bordered_box),
            container(right_column)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20)
        ];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Default for RustCuts {
    fn default() -> Self {
        let config_path = config::get_config_path(&None);
        let command_definitions = file_handling::get_command_definitions(&config_path)
            .unwrap_or_else(|_| Vec::new());
        Self {
            command_definitions,
            selected_command: None,
            parameter_values: HashMap::new(),
            output: None,
        }
    }
}

fn get_command_display_name(cmd: &CommandDefinition) -> String {
    match (&cmd.id, &cmd.description) {
        (Some(id), Some(desc)) => format!("{} ({})", id, desc),
        (Some(id), None) => id.clone(),
        (None, Some(desc)) => desc.clone(),
        (None, None) => cmd.command.join(" "),
    }
}

async fn run_in_terminal(
    cmd: CommandDefinition,
    params: HashMap<String, String>,
) -> Result<(), String> {
    let mut interpolated_command = Vec::new();
    
    for cmd_part in &cmd.command {
        let template = match Template::parse(cmd_part) {
            Ok(t) => t,
            Err(e) => return Err(format!("Template parse error: {}", e)),
        };
        
        let interpolated = match template.render(&params) {
            Ok(s) => s,
            Err(e) => return Err(format!("Template render error: {}", e)),
        };
        
        interpolated_command.push(interpolated);
    }

    if interpolated_command.is_empty() {
        return Err("No command to execute".to_string());
    }

    // Build the full command string for the terminal
    let mut full_command = interpolated_command.join(" ");
    
    // Handle working directory by prefixing with 'cd' if needed
    if let Some(cwd) = &cmd.working_directory {
        let template = match Template::parse(cwd) {
            Ok(t) => t,
            Err(e) => return Err(format!("CWD template parse error: {}", e)),
        };
        
        let interpolated_cwd = match template.render(&params) {
            Ok(s) => s,
            Err(e) => return Err(format!("CWD template render error: {}", e)),
        };
        
        full_command = format!("cd '{}' && {}", interpolated_cwd, full_command);
    }
    
    // Handle environment variables by prefixing with exports
    if let Some(env_vars) = &cmd.environment {
        let mut env_commands = Vec::new();
        for (key, value) in env_vars {
            let template = match Template::parse(value) {
                Ok(t) => t,
                Err(e) => return Err(format!("Environment variable template parse error: {}", e)),
            };
            
            let interpolated_value = match template.render(&params) {
                Ok(s) => s,
                Err(e) => return Err(format!("Environment variable template render error: {}", e)),
            };
            
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

async fn execute_command(
    cmd: CommandDefinition,
    params: HashMap<String, String>,
) -> Result<String, String> {
    let mut interpolated_command = Vec::new();
    
    for cmd_part in &cmd.command {
        let template = match Template::parse(cmd_part) {
            Ok(t) => t,
            Err(e) => return Err(format!("Template parse error: {}", e)),
        };
        
        let interpolated = match template.render(&params) {
            Ok(s) => s,
            Err(e) => return Err(format!("Template render error: {}", e)),
        };
        
        interpolated_command.push(interpolated);
    }

    let mut command = if interpolated_command.is_empty() {
        return Err("No command to execute".to_string());
    } else {
        let mut cmd = Command::new(&interpolated_command[0]);
        if interpolated_command.len() > 1 {
            cmd.args(&interpolated_command[1..]);
        }
        cmd
    };

    if let Some(cwd) = &cmd.working_directory {
        let template = match Template::parse(cwd) {
            Ok(t) => t,
            Err(e) => return Err(format!("CWD template parse error: {}", e)),
        };
        
        let interpolated_cwd = match template.render(&params) {
            Ok(s) => s,
            Err(e) => return Err(format!("CWD template render error: {}", e)),
        };
        
        command.current_dir(&interpolated_cwd);
    }

    if let Some(env_vars) = &cmd.environment {
        for (key, value) in env_vars {
            let template = match Template::parse(value) {
                Ok(t) => t,
                Err(e) => return Err(format!("Environment variable template parse error: {}", e)),
            };
            
            let interpolated_value = match template.render(&params) {
                Ok(s) => s,
                Err(e) => return Err(format!("Environment variable template render error: {}", e)),
            };
            
            command.env(key, interpolated_value);
        }
    }

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

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(RustCuts::title, RustCuts::update, RustCuts::view)
        .centered()
        .run()
}