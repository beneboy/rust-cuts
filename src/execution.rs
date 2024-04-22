use std::process::{Command, ExitCode, Stdio};

pub fn execute_command(mut command: Command) -> ExitCode {
    let spawn_result = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    let Ok(mut child) = spawn_result else {
        eprintln!("Error spawning subprocess: {}", spawn_result.unwrap_err());
        return ExitCode::FAILURE;
    };

    let child_process_result = child.wait();
    return match child_process_result {
        Ok(exit_status) => {
            if exit_status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("Subprocess didn't give a result: {}", e);
            return ExitCode::FAILURE;
        }
    };
}
