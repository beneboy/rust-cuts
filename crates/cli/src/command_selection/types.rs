use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use rust_cuts_core::command_definitions::{CommandDefinition, CommandExecutionTemplate};

/// Represents the user's command selection choice
pub enum CommandChoice {
    Index(usize),
    CommandId(String),
    Rerun(CommandExecutionTemplate),
    Quit,
}

/// Represents the user's choice when confirming a command run
pub enum RunChoice {
    Yes,
    No,
    ChangeParams,
}

/// Direction to cycle through commands
#[derive(Clone, Copy)]
pub enum CycleDirection {
    Up,
    Down,
}

/// Represents the type of command to display
pub enum CommandForDisplay {
    Normal(CommandDefinition),
    Rerun(CommandExecutionTemplate),
}

impl Display for CommandForDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandForDisplay::Normal(n) => write!(f, "{}", n),
            CommandForDisplay::Rerun(r) => write!(f, "{}", r),
        }
    }
}

/// Indexes for commands in the selection UI
#[derive(PartialEq, Eq, Hash, Clone)]
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
    pub fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (CommandIndex::Normal(i1), CommandIndex::Normal(i2)) => i1.cmp(i2),
            (_, CommandIndex::Normal(_)) => Ordering::Greater,
            (CommandIndex::Normal(_), _) => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
}

/// State for the UI viewport
pub struct ViewportState {
    pub offset: usize,
    pub height: u16,
    pub width: u16,
}

/// Mode for the display UI
pub struct DisplayMode {
    pub is_filtering: bool,
}