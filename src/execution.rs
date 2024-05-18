use std::collections::HashMap;
use std::process::{Command, Stdio};

use log::info;

use crate::error::{Error, Result};

pub fn execute_command(mut command: Command, environment: Option<HashMap<String, String>>) -> Result<()> {
    let mut command = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(environment) = environment {
        info!("Executing with environment variables: {:?}", environment);
        command = command.envs(environment)
    };

    let subprocess_exit_success = command.spawn()?
        .wait()?
        .success();

    if subprocess_exit_success {
        Ok(())
    } else {
        Err(Error::SubProcessExit)
    }
}
