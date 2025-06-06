use std::collections::HashMap;

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
