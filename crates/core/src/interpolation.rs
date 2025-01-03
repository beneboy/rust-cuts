use std::collections::{HashMap, HashSet};

use leon::Template;

use crate::command_definitions::ParameterDefinition;
use crate::error::Result;

pub fn build_default_lookup(
    definitions: &Option<Vec<ParameterDefinition>>,
) -> Option<HashMap<String, String>> {
    if let Some(definitions) = definitions.as_ref() {
        let mut defaults: HashMap<String, String> = HashMap::new();
        for definition in definitions {
            if let Some(default) = &definition.default {
                defaults.insert(definition.name.clone(), default.clone());
            }
        }

        Some(defaults)
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
    context: &Option<HashMap<String, String>>,
    templates: &[Template],
) -> Result<Vec<String>> {
    let mut interpolated_arguments: Vec<String> = Vec::new();

    let empty_hashmap: HashMap<String, String> = HashMap::new();

    let context = context.as_ref().unwrap_or(&empty_hashmap);

    for template in templates {
        interpolated_arguments.push(template.render(&context)?);
    }

    Ok(interpolated_arguments)
}
