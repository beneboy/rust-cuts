use crate::app::{Message, ExecutionState};
use crate::execution::{execute_command, run_in_terminal};
use iced::{Element, Task, Subscription, event, keyboard, widget::text_input};
use rust_cuts_core::command_definitions::{CommandDefinition, TemplateParser};
use rust_cuts_core::{config, file_handling};
use std::collections::HashMap;

pub struct RustCuts {
    pub command_definitions: Vec<CommandDefinition>,
    pub selected_command: Option<usize>,
    pub parameter_values: HashMap<String, String>,
    pub output: Option<Result<String, String>>,
    pub streaming_output: String, // Accumulate streaming output
    pub current_parameters: Vec<String>, // Track parameter order for focus management
    pub focused_parameter_index: Option<usize>, // Track which parameter is currently focused
    pub execution_state: ExecutionState, // Track if commands are running
    pub progress_counter: usize, // For animated progress dots
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
                self.streaming_output.clear();
                self.current_parameters.clear();
                self.focused_parameter_index = None;
                self.progress_counter = 0;
                
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
                        for var_name in &template_vars {
                            if !param_lookup.contains_key(var_name) {
                                // Create a basic parameter definition for undefined template vars
                                param_lookup.insert(var_name.clone(), rust_cuts_core::command_definitions::ParameterDefinition {
                                    id: var_name.clone(),
                                    default: None,
                                    description: None,
                                });
                            }
                            
                            // Set default values in parameter_values
                            let default_value = param_lookup.get(var_name)
                                .and_then(|p| p.default.clone())
                                .unwrap_or_default();
                            self.parameter_values.insert(var_name.clone(), default_value);
                        }
                        
                        // Store parameter order for focus management
                        self.current_parameters = template_vars.into_iter().collect();
                        
                        // Focus first parameter if any exist
                        if !self.current_parameters.is_empty() {
                            self.focused_parameter_index = Some(0);
                            let first_param_id = text_input::Id::new(format!("param_0_{}", self.current_parameters[0]));
                            return text_input::focus(first_param_id);
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
                if self.execution_state != ExecutionState::Idle {
                    return Task::none(); // Don't start new task if already running
                }
                
                if let Some(index) = self.selected_command {
                    if let Some(cmd) = self.command_definitions.get(index).cloned() {
                        self.execution_state = ExecutionState::RunningInline;
                        self.output = None; // Clear previous output
                        self.streaming_output.clear(); // Clear streaming output
                        self.progress_counter = 0; // Reset progress animation
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
                        self.execution_state = ExecutionState::RunningInTerminal;
                        let params = self.parameter_values.clone();
                        return Task::perform(
                            async move { run_in_terminal(cmd, params).await },
                            Message::TerminalLaunched,
                        );
                    }
                }
                Task::none()
            }
            Message::CancelCommand => {
                if self.execution_state == ExecutionState::RunningInline {
                    self.execution_state = ExecutionState::Idle;
                    self.output = Some(Err("Command cancelled by user".to_string()));
                    self.streaming_output.clear();
                }
                // Note: Terminal commands can't be cancelled
                Task::none()
            }
            Message::CommandOutputUpdate(new_output) => {
                self.streaming_output.push_str(&new_output);
                Task::none()
            }
            Message::CommandExecuted(result) => {
                self.execution_state = ExecutionState::Idle; // Reset state when command completes
                self.output = Some(result);
                Task::none()
            }
            Message::TerminalLaunched(result) => {
                self.execution_state = ExecutionState::Idle; // Reset state when terminal launches
                // Only show output if there was an error
                if let Err(e) = result {
                    self.output = Some(Err(format!("Failed to launch terminal: {}", e)));
                }
                // Don't show any output for successful terminal launches
                Task::none()
            }
            Message::FocusNext => {
                if let Some(current_index) = self.focused_parameter_index {
                    let next_index = (current_index + 1) % self.current_parameters.len();
                    self.focused_parameter_index = Some(next_index);
                    
                    if let Some(param_name) = self.current_parameters.get(next_index) {
                        let param_id = text_input::Id::new(format!("param_{}_{}", next_index, param_name));
                        return text_input::focus(param_id);
                    }
                }
                Task::none()
            }
            Message::FocusPrevious => {
                if let Some(current_index) = self.focused_parameter_index {
                    let prev_index = if current_index == 0 {
                        self.current_parameters.len().saturating_sub(1)
                    } else {
                        current_index - 1
                    };
                    self.focused_parameter_index = Some(prev_index);
                    
                    if let Some(param_name) = self.current_parameters.get(prev_index) {
                        let param_id = text_input::Id::new(format!("param_{}_{}", prev_index, param_name));
                        return text_input::focus(param_id);
                    }
                }
                Task::none()
            }
            Message::ProgressTick => {
                // Increment progress counter for spinner animation
                if self.execution_state == ExecutionState::RunningInline {
                    self.progress_counter = self.progress_counter.wrapping_add(1);
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        crate::ui::views::main_view(self)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_events = event::listen_with(|event, _status, _window| {
            match event {
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Tab),
                    modifiers,
                    ..
                }) => {
                    Some(if modifiers.shift() {
                        Message::FocusPrevious
                    } else {
                        Message::FocusNext
                    })
                }
                _ => None,
            }
        });

        let progress_ticker = if self.execution_state == ExecutionState::RunningInline {
            iced::time::every(std::time::Duration::from_millis(16)) // 60 FPS
                .map(|_| Message::ProgressTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([keyboard_events, progress_ticker])
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
            streaming_output: String::new(),
            current_parameters: Vec::new(),
            focused_parameter_index: None,
            execution_state: ExecutionState::Idle,
            progress_counter: 0,
        }
    }
}