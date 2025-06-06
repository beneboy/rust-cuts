use std::collections::HashMap;
use std::process::{Command, Stdio};

use log::info;

use crate::error::{Error, Result};

/// Executes a command with optional environment variables.
///
/// # Errors
///
/// Returns an error if command execution fails or exits with non-zero status.
pub fn execute_command<S: ::std::hash::BuildHasher>(
    mut command: Command,
    environment: Option<HashMap<String, String, S>>,
) -> Result<()> {
    let mut command = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(environment) = environment {
        info!("Executing with environment variables: {:?}", environment);
        command = command.envs(environment);
    };

    let subprocess_exit_success = command.spawn()?.wait()?.success();

    if subprocess_exit_success {
        Ok(())
    } else {
        Err(Error::SubProcessExit)
    }
}
