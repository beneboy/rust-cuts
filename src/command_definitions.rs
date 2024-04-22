use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Debug)]
pub struct ParameterDefinition {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LastCommandParameters {
    pub command: String,
    pub working_directory: Option<String>
}

impl CommandDefinition {
    pub fn to_string(&self) -> String {
        match &self.name {
            Some(name) => name.clone(),
            None => {
                self.command.join(" ")
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CommandDefinition {
    pub command: Vec<String>,
    pub name: Option<String>,
    pub working_directory: Option<String>,
    pub parameters: Option<Vec<ParameterDefinition>>,
}

impl Display for CommandDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())?;
        Ok(())
    }
}
