use std::process::{Command, Stdio};

use rc::error::{Error, Result};

use crate::rc;

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
        Err(Error::SubProcessExitFailure)
    }
}
