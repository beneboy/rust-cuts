use clap::Parser;
use rust_cuts_core::error::Result;
use rust_cuts_core::error::Error::MixedParameterMode;

#[derive(Parser, Debug)] // requires `derive` feature
#[command(term_width = 0)] // Just to make testing across clap features easier
#[allow(clippy::struct_excessive_bools)] // silence clippy's warning on this struct
pub struct Args {
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

    /// The command ID to execute directly (if not provided, interactive mode is used)
    #[arg(num_args(1))]
    pub command_id_or_index: Option<String>,

    /// Parameters for the command in the format key=value
    #[arg(long = "param", short = 'p', action = clap::ArgAction::Append)]
    pub parameters: Vec<String>,

    /// Positional parameters for substitution in the command template
    #[arg(trailing_var_arg = true)]
    pub positional_args: Vec<String>,
}

#[derive(PartialEq)]
pub enum ParameterMode {
    /// No parameters provided, will use default values or prompt interactively
    None,
    /// Named parameters provided with -p/--param flags (key=value format)
    Named(Vec<String>),
    /// Positional parameters provided as trailing arguments
    Positional(Vec<String>),
}

impl Args {
    /// Validates that named and positional parameters aren't mixed
    pub fn get_parameter_mode(&self) -> Result<ParameterMode> {
        let using_named = !self.parameters.is_empty();
        let using_positional = !self.positional_args.is_empty();

        match (using_named, using_positional) {
            (true, true) => Err(MixedParameterMode),
            (true, false) => Ok(ParameterMode::Named(self.parameters.clone())),
            (false, true) => Ok(ParameterMode::Positional(self.positional_args.clone())),
            (false, false) => Ok(ParameterMode::None),
        }
    }
}