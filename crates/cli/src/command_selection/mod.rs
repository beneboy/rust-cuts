// Export public items from submodules
pub mod types;
pub mod ui;
pub mod input;
pub mod colors;

// Re-exports for convenience
pub use types::{CommandChoice, RunChoice};
pub use input::{fill_parameter_values, prompt_value, confirm_command_should_run};
pub use ui::prompt_for_command_choice;

// Any module-level constants
pub const LAST_COMMAND_OPTION: char = 'r';