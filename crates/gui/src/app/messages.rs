#[derive(Debug, Clone)]
pub enum Message {
    CommandSelected(usize),
    ParameterChanged(String, String),
    RunCommand,
    RunInTerminal,
    CancelCommand,
    CommandOutputUpdate(String), // For streaming output
    CommandExecuted(Result<String, String>),
    TerminalLaunched(Result<(), String>),
    ProgressTick, // For animated progress indication
    FocusNext,
    FocusPrevious,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    Idle,
    RunningInline,
    RunningInTerminal,
}