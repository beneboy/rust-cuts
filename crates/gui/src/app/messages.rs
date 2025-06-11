#[derive(Debug, Clone)]
pub enum Message {
    CommandSelected(usize),
    ParameterChanged(String, String),
    RunCommand,
    RunInTerminal,
    CommandExecuted(Result<String, String>),
    TerminalLaunched(Result<(), String>),
}