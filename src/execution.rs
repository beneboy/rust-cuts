use std::process::{Command, Stdio};

use crate::error::{Error, Result};


pub fn execute_command(mut command: Command) -> Result<()> {
    let subprocess_exit_success = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?
        .wait()?
        .success();

    if subprocess_exit_success {
        Ok(())
    } else {
        Err(Error::SubProcessExit)
    }
}
