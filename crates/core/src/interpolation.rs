use std::collections::HashMap;

use leon::Template;

use crate::command_definitions::ParameterDefinition;
use crate::error::Result;

#[must_use]
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

/// Interpolates template commands with provided context values.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn interpolate_command<S: ::std::hash::BuildHasher>(
    context: &HashMap<String, String, S>,
    templates: &[Template],
) -> Result<Vec<String>> {
    let mut interpolated_arguments: Vec<String> = Vec::new();

    for template in templates {
        interpolated_arguments.push(template.render(&context)?);
    }

    Ok(interpolated_arguments)
}
