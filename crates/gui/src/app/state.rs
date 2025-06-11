use crate::app::Message;
use crate::execution::{execute_command, run_in_terminal};
use iced::{Element, Task};
use rust_cuts_core::command_definitions::{CommandDefinition, TemplateParser};
use rust_cuts_core::{config, file_handling};
use std::collections::HashMap;

pub struct RustCuts {
    pub command_definitions: Vec<CommandDefinition>,
    pub selected_command: Option<usize>,
    pub parameter_values: HashMap<String, String>,
    pub output: Option<Result<String, String>>,
}

impl RustCuts {
    pub fn title(&self) -> String {
        "RustCuts GUI".to_string()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CommandSelected(index) => {
                self.selected_command = Some(index);
                self.parameter_values.clear();
                self.output = None;
                if let Some(cmd) = self.command_definitions.get(index) {
                    // Extract all template variables from command strings
                    if let Ok(template_vars) = cmd.get_ordered_context_variables() {
                        // Create parameter definitions, merging explicit params with template vars
                        let mut param_lookup: HashMap<String, rust_cuts_core::command_definitions::ParameterDefinition> = HashMap::new();
                        
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
                                param_lookup.insert(var_name.clone(), rust_cuts_core::command_definitions::ParameterDefinition {
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

    pub fn view(&self) -> Element<Message> {
        crate::ui::views::main_view(self)
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