mod app;
mod execution;
mod ui;
mod utils;

use app::RustCuts;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(RustCuts::title, RustCuts::update, RustCuts::view)
        .subscription(RustCuts::subscription)
        .centered()
        .run()
}