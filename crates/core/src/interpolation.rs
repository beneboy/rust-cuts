use std::collections::{HashMap, HashSet};

use leon::Template;

use crate::command_definitions::ParameterDefinition;
use crate::error::Result;

pub fn build_parameter_lookup(
    definitions: &Option<Vec<ParameterDefinition>>,
) -> Option<HashMap<String, ParameterDefinition>> {
    if let Some(definitions) = definitions.as_ref() {
        let mut parameter_definitions: HashMap<String, ParameterDefinition> = HashMap::new();
        for definition in definitions {
            parameter_definitions.insert(definition.id.clone(), definition.clone());
        }

        Some(parameter_definitions)
    } else {
        None
    }
}

/// Find all tokens in all arguments of templates of command.
pub fn get_tokens(templates: &[Template]) -> HashSet<String> {
    let mut tokens = HashSet::new();

    for template in templates {
        for key in template.keys() {
            let _ = tokens.insert((*key).to_string());
        }
    }

    tokens
}

pub fn get_templates(command: &[String]) -> Result<Vec<Template>> {
    let mut templates: Vec<Template> = Vec::new();

    for argument in command {
        templates.push(Template::parse(argument.as_ref())?);
    }

    Ok(templates)
}

pub fn interpolate_command(
    context: &HashMap<String, String>,
    templates: &[Template],
) -> Result<Vec<String>> {
    let mut interpolated_arguments: Vec<String> = Vec::new();

    for template in templates {
        interpolated_arguments.push(template.render(&context)?);
    }

    Ok(interpolated_arguments)
}
