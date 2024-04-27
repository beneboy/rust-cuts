use std::collections::HashMap;
use leon::Template;
use crate::command_definitions::ParameterDefinition;
use crate::command_selection;

pub fn build_default_lookup(definitions: &Option<Vec<ParameterDefinition>>) -> Option<HashMap<String, String>> {
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

pub fn interpolate_arguments(defaults: &Option<HashMap<String, String>>, arguments: &[String]) -> Vec<String> {
    let mut interpolated_arguments: Vec<String> = vec![];
    for argument in arguments.iter() {
        match Template::parse(argument.as_ref()) {
            Ok(template) => {
                let mut context: HashMap<String, String> = HashMap::new();
                for key in template.keys() {
                    let default_value = match defaults {
                        Some(defaults) => { defaults.get(&key.to_string()) }
                        None => { None }
                    };

                    let value = command_selection::prompt_value(key, default_value.cloned());

                    context.insert(key.parse().unwrap(), value);
                }

                if context.is_empty() {
                    interpolated_arguments.push(argument.clone())
                } else {
                    match template.render(&context) {
                        Ok(rendered_argument) => {
                            interpolated_arguments.push(rendered_argument)
                        }
                        Err(err) => {
                            panic!("{}", err)
                        }
                    }
                }
            }
            Err(err) => {
                panic!("{}", err);
            }
        }
    }
    interpolated_arguments
}
