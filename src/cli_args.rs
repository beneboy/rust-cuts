use clap::Parser;

#[derive(Parser, Debug)] // requires `derive` feature
#[command(term_width = 0)] // Just to make testing across clap features easier
pub(crate) struct Args {
    /// Path to the commands definition config file YAML.
    #[arg(long, short = 'c')]
    pub config_path: Option<String>,

    /// Path to the file that stores the interpolated last command.
    #[arg(long, short = 'l')]
    pub last_command_path: Option<String>,

    /// Perform a dry run, which just prints out the command but does not execute it.
    #[arg(long, short = 'd', action)]
    pub dry_run: bool,

    /// Run the command without first confirming if the command should be run.
    #[arg(long, short = 'f', action)]
    pub force: bool,

    /// Rerun the last command (do not show allow to select another).
    #[arg(long, short = 'r', action)]
    pub rerun_last_command: bool,

    /// Skip saving of this command as the last command to replay. Retains existing last command.
    #[arg(long, short = 's', action)]
    pub skip_command_save: bool,
}