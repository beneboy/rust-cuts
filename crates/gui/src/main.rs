use iced::widget::{container, text, Column, Row};
use iced::{Element, Length};
use rust_cuts_core::command_definitions::CommandDefinition;
use rust_cuts_core::{config, file_handling};

#[derive(Debug, Clone)]
pub enum Message {}

struct RustCuts {
    command_definitions: Vec<CommandDefinition>,
}


impl RustCuts {
    fn title(&self) -> String {
        "RustCuts GUI".to_string()
    }

    fn update(&mut self, _event: Message) {}

    fn view(&self) -> Element<Message> {
        // Left column - fixed width
        let left_column = self.command_definitions.iter().fold(
            Column::new(),
            |column, cd| {
                let command_name = format!("{}", cd);
                let row = Row::new()
                    .push(text(command_name).width(Length::Fill).height(Length::Fixed(40.0)));
                column.push(row)
            }
        );

        // Right column - fills remaining space
        let right_column = Column::new()
            .push(text("Right Column"))
            .width(Length::Fill)
            .height(Length::Fill);

        // Main row containing both columns
        let content = Row::new()
            .push(
                container(left_column)
                    .width(Length::Fixed(200.0))
                    .height(Length::Fill)
            )
            .push(
                container(right_column)
                    .width(Length::Fill)
                    .height(Length::Fill)
            );

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }


}

impl Default for RustCuts {
    fn default() -> Self {
        let config_path = config::get_config_path(&None);
        let command_definitions = file_handling::get_command_definitions(&config_path).expect("Failed to load config.");
        Self {
            command_definitions
        }
    }
}

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(RustCuts::title, RustCuts::update, RustCuts::view)
        .centered()
        .run()
}