use crate::app::{Message, RustCuts};
use crate::ui::components;
use iced::widget::{container, row, scrollable, text, Column};
use iced::{Center, Element, Length};
use rust_cuts_core::command_definitions::TemplateParser;
use std::collections::HashMap;

pub fn main_view(app: &RustCuts) -> Element<Message> {
    let left_column = command_list_view(&app.command_definitions, app.selected_command);
    let right_column = command_details_view(app);

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

fn command_list_view(
    commands: &[rust_cuts_core::command_definitions::CommandDefinition],
    selected_command: Option<usize>,
) -> Element<Message> {
    let mut column = Column::new().spacing(5).padding(10);

    for (index, cmd) in commands.iter().enumerate() {
        let is_selected = selected_command == Some(index);
        let btn = components::command_button(cmd, index, is_selected);
        column = column.push(btn);
    }

    scrollable(column).height(Length::Fill).into()
}

fn command_details_view(app: &RustCuts) -> Element<Message> {
    if let Some(index) = app.selected_command {
        if let Some(cmd) = app.command_definitions.get(index) {
            let mut details = Column::new()
                .spacing(15)
                .padding(20)
                .push(text(crate::utils::display::get_command_display_name(cmd)).size(24))
                .push(text(cmd.command.join(" ")).size(14));

            // Show parameter fields for all template variables
            if let Ok(template_vars) = cmd.get_ordered_context_variables() {
                if !template_vars.is_empty() {
                    details = details.push(text("Parameters:").size(18));

                    // Create parameter lookup from explicit definitions
                    let mut param_lookup: HashMap<String, rust_cuts_core::command_definitions::ParameterDefinition> = HashMap::new();
                    if let Some(params) = &cmd.parameters {
                        for param in params {
                            param_lookup.insert(param.id.clone(), param.clone());
                        }
                    }

                    for (tab_index, var_name) in template_vars.iter().enumerate() {
                        let value = app
                            .parameter_values
                            .get(var_name)
                            .cloned()
                            .unwrap_or_default();

                        let description = param_lookup.get(var_name)
                            .and_then(|p| p.description.as_deref());

                        let param_input = components::parameter_input(var_name, &value, description, tab_index);
                        details = details.push(param_input);
                    }
                }
            }

            details = details.push(components::action_buttons(&app.execution_state));

            // Always show output area (even if empty) at full width
            let output_content = if app.execution_state == crate::app::ExecutionState::RunningInline && !app.streaming_output.is_empty() {
                // Show streaming output while command is running
                text(&app.streaming_output).size(14)
            } else if let Some(output) = &app.output {
                // Show final output when command is complete
                match output {
                    Ok(stdout) => text(stdout).size(14),
                    Err(error) => text(error).size(14).color([0.8, 0.2, 0.2]),
                }
            } else {
                text("")
            };
            
            details = details.push(
                container(scrollable(output_content))
                    .padding(10)
                    .style(container::bordered_box)
                    .width(Length::Fill)
                    .height(Length::FillPortion(3)), // Make it larger
            );

            details.into()
        } else {
            Column::new().into()
        }
    } else {
        Column::new()
            .push(text("Select a command from the list").size(18))
            .align_x(Center)
            .into()
    }
}