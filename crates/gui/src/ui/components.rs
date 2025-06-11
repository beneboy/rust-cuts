use crate::app::{Message, ExecutionState};
use iced::widget::{button, text, text_input, svg, Column};
use iced::{Length, widget::text_input::Id, Rotation};
use rust_cuts_core::command_definitions::CommandDefinition;

// Single SVG spinner - a circle with a partial arc
const SPINNER_SVG: &[u8] = b"<svg width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" xmlns=\"http://www.w3.org/2000/svg\"><circle cx=\"12\" cy=\"12\" r=\"10\" fill=\"none\" stroke=\"gray\" stroke-width=\"2\" opacity=\"0.3\"/><path d=\"M12 2 A10 10 0 0 1 22 12\" fill=\"none\" stroke=\"black\" stroke-width=\"2\" stroke-linecap=\"round\"/></svg>";

pub fn command_button(cmd: &CommandDefinition, index: usize, is_selected: bool) -> button::Button<Message> {
    let name = crate::utils::display::get_command_display_name(cmd);
    button(text(name).size(16))
        .width(Length::Fill)
        .padding(10)
        .style(if is_selected {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::CommandSelected(index))
}

pub fn parameter_input(
    key: &str,
    value: &str,
    description: Option<&str>,
    tab_index: usize,
) -> Column<'static, Message> {
    let key_owned = key.to_string();
    let value_owned = value.to_string();
    let description_owned = description.map(|s| s.to_string());
    
    // Create a unique ID for this input field for tab navigation
    // Use both tab_index and key to ensure uniqueness
    let input_id = Id::new(format!("param_{}_{}", tab_index, key_owned));
    
    let input = text_input(&format!("Enter {}", key_owned), &value_owned)
        .id(input_id)
        .on_input({
            let key = key_owned.clone();
            move |val| Message::ParameterChanged(key.clone(), val)
        })
        .padding(8)
        .size(16)
        .width(Length::Fill);

    let mut param_column = Column::new()
        .spacing(5)
        .push(text(key_owned).size(14))
        .push(input);

    if let Some(desc) = description_owned {
        param_column = param_column.push(text(desc).size(12));
    }

    param_column
}

pub fn action_buttons(execution_state: &ExecutionState, progress_counter: usize) -> iced::widget::Row<'static, Message> {
    let mut buttons = iced::widget::Row::new().spacing(10);
    
    match execution_state {
        ExecutionState::Idle => {
            // Normal state - show active run buttons and disabled cancel
            buttons = buttons
                .push(
                    button(text("Run").size(16))
                        .padding([10, 20])
                        .style(button::success)
                        .on_press(Message::RunCommand)
                )
                .push(
                    button(text("Run In Terminal").size(16))
                        .padding([10, 20])
                        .style(button::primary)
                        .on_press(Message::RunInTerminal)
                )
                .push(
                    button(text("Cancel").size(16))
                        .padding([10, 20])
                        .style(button::secondary) // Disabled style
                        // No on_press - disabled
                );
        }
        ExecutionState::RunningInline => {
            // Inline execution - show disabled run buttons and active cancel button
            buttons = buttons
                .push(
                    button(text("Running...").size(16))
                        .padding([10, 20])
                        .style(button::secondary) // Disabled style
                )
                .push(
                    button(text("Run In Terminal").size(16))
                        .padding([10, 20])
                        .style(button::secondary) // Disabled style
                )
                .push(
                    button(text("Cancel").size(16))
                        .padding([10, 20])
                        .style(button::danger)
                        .on_press(Message::CancelCommand)
                );
            
            // Add SVG spinner widget to the right of the cancel button
            // Full rotation (360°) in 2 seconds: 120 ticks × 16ms = 1920ms ≈ 2 seconds
            // So 360° ÷ 120 ticks = 3° per tick
            let rotation_radians = progress_counter as f32 * 3.0 * std::f32::consts::PI / 180.0;
            let spinner_svg = svg::Handle::from_memory(SPINNER_SVG);
            buttons = buttons.push(
                svg(spinner_svg)
                    .width(20)
                    .height(20)
                    .rotation(rotation_radians)
            );
        }
        ExecutionState::RunningInTerminal => {
            // Terminal execution - show disabled buttons and disabled cancel
            buttons = buttons
                .push(
                    button(text("Run").size(16))
                        .padding([10, 20])
                        .style(button::secondary) // Disabled style
                )
                .push(
                    button(text("Launched in Terminal").size(16))
                        .padding([10, 20])
                        .style(button::secondary) // Disabled style
                )
                .push(
                    button(text("Cancel").size(16))
                        .padding([10, 20])
                        .style(button::secondary) // Disabled style
                        // No on_press - can't cancel terminal commands
                );
        }
    }
    
    buttons
}