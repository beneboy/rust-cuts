//! Type definitions for command selection and UI state.
//!
//! This module defines the core types used throughout the command selection
//! interface, including user choices, display modes, and UI state management.

use rust_cuts_core::command_definitions::{CommandDefinition, CommandExecutionTemplate};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

/// Represents the user's command selection choice.
///
/// This enum captures the different ways a user can select a command
/// for execution, whether through the interactive UI or direct specification.
pub enum CommandChoice {
    Index(usize),
    CommandId(String),
    Rerun(CommandExecutionTemplate),
    Quit,
}

/// Represents the user's choice when confirming a command run.
///
/// After a command is selected and parameters are filled, the user
/// is prompted to confirm execution with these options.
pub enum RunChoice {
    Yes,
    No,
    ChangeParams,
}

/// Direction to cycle through commands in the selection UI.
#[derive(Clone, Copy)]
pub enum CycleDirection {
    Up,
    Down,
}

/// Represents the type of command to display in the selection UI.
///
/// Commands can be either normal command definitions from the configuration
/// or a special "rerun last command" option.
pub enum CommandForDisplay {
    Normal(CommandDefinition),
    Rerun(CommandExecutionTemplate),
}

impl Display for CommandForDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandForDisplay::Normal(n) => write!(f, "{n}"),
            CommandForDisplay::Rerun(r) => write!(f, "{r}"),
        }
    }
}

/// Indexes for commands in the selection UI.
///
/// Commands can be indexed either by their position in the normal command
/// list or as the special "rerun" option.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum CommandIndex {
    Normal(usize),
    Rerun,
}

impl Display for CommandIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandIndex::Normal(i) => f.write_str(format!("{}", i + 1).as_str()),
            CommandIndex::Rerun => f.write_str("r"),
        }
    }
}

impl CommandIndex {
    #[must_use]
    pub fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (CommandIndex::Normal(i1), CommandIndex::Normal(i2)) => i1.cmp(i2),
            (_, CommandIndex::Normal(_)) => Ordering::Greater,
            (CommandIndex::Normal(_), _) => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
}

/// State for the UI viewport.
///
/// Tracks the visible portion of the command list when there are more
/// commands than can fit on screen.
#[derive(Clone, PartialEq, Debug)]
pub struct ViewportState {
    pub offset: usize,
    pub height: u16,
    pub width: u16,
}

/// Mode for the display UI.
///
/// Controls whether the interface is in filtering mode or normal selection mode.
pub struct DisplayMode {
    pub is_filtering: bool,
}

/// Complete UI state for the command selection interface.
///
/// Maintains all state needed to render and interact with the command
/// selection UI, including current selection, viewport, and filter state.
#[derive(Clone, PartialEq, Debug)]
pub struct UiState {
    /// Currently selected command index
    pub selected_index: usize,
    /// Viewport state for scrolling
    pub viewport: ViewportState,
    /// Whether the user is currently filtering/searching
    pub is_filtering: bool,
    /// Current filter/search text
    pub filter_text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_cuts_core::command_definitions::CommandDefinition;

    fn create_test_command() -> CommandDefinition {
        CommandDefinition {
            command: vec!["echo".to_string(), "test".to_string()],
            id: Some("test_cmd".to_string()),
            description: Some("Test command".to_string()),
            working_directory: None,
            parameters: None,
            environment: None,
            metadata: None,
        }
    }

    fn create_test_execution_template() -> CommandExecutionTemplate {
        CommandExecutionTemplate {
            command: vec!["ls".to_string(), "-la".to_string()],
            working_directory: None,
            template_context: None,
            environment: None,
        }
    }

    #[test]
    fn test_command_for_display_normal() {
        let cmd = create_test_command();
        let display_cmd = CommandForDisplay::Normal(cmd.clone());
        let display_str = format!("{display_cmd}");
        assert_eq!(display_str, "test_cmd (Test command)");
    }

    #[test]
    fn test_command_for_display_rerun() {
        let template = create_test_execution_template();
        let display_cmd = CommandForDisplay::Rerun(template);
        let display_str = format!("{display_cmd}");
        assert_eq!(display_str, "ls -la");
    }

    #[test]
    fn test_command_index_display() {
        let normal_index = CommandIndex::Normal(0);
        assert_eq!(format!("{normal_index}"), "1"); // 1-based display

        let normal_index_5 = CommandIndex::Normal(4);
        assert_eq!(format!("{normal_index_5}"), "5");

        let rerun_index = CommandIndex::Rerun;
        assert_eq!(format!("{rerun_index}"), "r");
    }

    #[test]
    fn test_command_index_compare() {
        let normal_1 = CommandIndex::Normal(0);
        let normal_2 = CommandIndex::Normal(1);
        let rerun = CommandIndex::Rerun;

        // Normal indices compare by value
        assert_eq!(normal_1.compare(&CommandIndex::Normal(0)), Ordering::Equal);
        assert_eq!(normal_1.compare(&normal_2), Ordering::Less);
        assert_eq!(normal_2.compare(&normal_1), Ordering::Greater);

        // Rerun comes after normal indices
        assert_eq!(normal_1.compare(&rerun), Ordering::Less);
        assert_eq!(rerun.compare(&normal_1), Ordering::Greater);

        // Rerun equals rerun
        assert_eq!(rerun.compare(&CommandIndex::Rerun), Ordering::Equal);
    }

    #[test]
    fn test_command_index_equality() {
        let normal_first = CommandIndex::Normal(1);
        let normal_same = CommandIndex::Normal(1);
        let normal_different = CommandIndex::Normal(2);
        let rerun_first = CommandIndex::Rerun;
        let rerun_second = CommandIndex::Rerun;

        assert_eq!(normal_first, normal_same);
        assert_ne!(normal_first, normal_different);
        assert_eq!(rerun_first, rerun_second);
        assert_ne!(normal_first, rerun_first);
    }

    #[test]
    fn test_cycle_direction() {
        // Test that CycleDirection can be copied
        let up = CycleDirection::Up;
        let up_copy = up;

        let down = CycleDirection::Down;
        let down_copy = down;

        // Verify they can be used after copying (tests Copy trait)
        match up_copy {
            CycleDirection::Up => {}
            CycleDirection::Down => panic!("Expected Up"),
        }

        match down_copy {
            CycleDirection::Up => panic!("Expected Down"),
            CycleDirection::Down => {}
        }
    }

    #[test]
    fn test_viewport_state_equality() {
        let viewport1 = ViewportState {
            offset: 0,
            height: 10,
            width: 80,
        };

        let viewport2 = ViewportState {
            offset: 0,
            height: 10,
            width: 80,
        };

        let viewport3 = ViewportState {
            offset: 1,
            height: 10,
            width: 80,
        };

        assert_eq!(viewport1, viewport2);
        assert_ne!(viewport1, viewport3);
    }

    #[test]
    fn test_ui_state_equality() {
        let viewport = ViewportState {
            offset: 0,
            height: 10,
            width: 80,
        };

        let ui_state1 = UiState {
            selected_index: 0,
            viewport: viewport.clone(),
            is_filtering: false,
            filter_text: String::new(),
        };

        let ui_state2 = UiState {
            selected_index: 0,
            viewport: viewport.clone(),
            is_filtering: false,
            filter_text: String::new(),
        };

        let ui_state3 = UiState {
            selected_index: 1, // Different selected index
            viewport,
            is_filtering: false,
            filter_text: String::new(),
        };

        assert_eq!(ui_state1, ui_state2);
        assert_ne!(ui_state1, ui_state3);
    }

    #[test]
    fn test_display_mode() {
        let filtering_mode = DisplayMode { is_filtering: true };
        let normal_mode = DisplayMode {
            is_filtering: false,
        };

        assert!(filtering_mode.is_filtering);
        assert!(!normal_mode.is_filtering);
    }
}
