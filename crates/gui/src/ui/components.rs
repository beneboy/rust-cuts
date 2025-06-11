use crate::app::Message;
use iced::widget::{button, text, text_input, Column};
use iced::Length;
use rust_cuts_core::command_definitions::CommandDefinition;

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
) -> Column<'static, Message> {
    let key_owned = key.to_string();
    let value_owned = value.to_string();
    let description_owned = description.map(|s| s.to_string());
    
    let input = text_input(&format!("Enter {}", key_owned), &value_owned)
        .on_input({
            let key = key_owned.clone();
            move |val| Message::ParameterChanged(key.clone(), val)
        })
        .padding(8)
        .size(16);

    let mut param_column = Column::new()
        .spacing(5)
        .push(text(key_owned).size(14))
        .push(input);

    if let Some(desc) = description_owned {
        param_column = param_column.push(text(desc).size(12));
    }

    param_column
}

pub fn action_buttons() -> iced::widget::Row<'static, Message> {
    iced::widget::row![
        button(text("Run").size(16))
            .padding([10, 20])
            .style(button::success)
            .on_press(Message::RunCommand),
        button(text("Run In Terminal").size(16))
            .padding([10, 20])
            .style(button::primary)
            .on_press(Message::RunInTerminal),
    ]
    .spacing(10)
}