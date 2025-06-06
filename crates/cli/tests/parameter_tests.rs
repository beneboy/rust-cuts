#[cfg(test)]
mod tests {
    use indexmap::IndexSet;
    use rust_cuts_cli::parameters::mode::{determine_parameter_mode, ParameterMode};
    use rust_cuts_cli::parameters::validation::should_prompt_for_parameters;
    use rust_cuts_core::command_definitions::ParameterDefinition;
    use std::collections::HashMap;

    #[test]
    fn test_parameter_mode_determination() {
        // Test with no parameters
        let empty_named: Vec<String> = vec![];
        let empty_positional: Vec<String> = vec![];
        let mode = determine_parameter_mode(&empty_named, &empty_positional).unwrap();
        assert!(matches!(mode, ParameterMode::None));

        // Test with named parameters
        let named = vec!["key1=value1".to_string(), "key2=value2".to_string()];
        let mode = determine_parameter_mode(&named, &empty_positional).unwrap();
        assert!(matches!(mode, ParameterMode::Named(_)));

        // Test with positional parameters
        let positional = vec!["value1".to_string(), "value2".to_string()];
        let mode = determine_parameter_mode(&empty_named, &positional).unwrap();
        assert!(matches!(mode, ParameterMode::Positional(_)));

        // Test mixed parameters (should error)
        let result = determine_parameter_mode(&named, &positional);
        assert!(result.is_err());
    }

    #[test]
    fn test_should_prompt_for_parameters() {
        let tokens: IndexSet<String> = ["token1", "token2"].iter().map(|s| s.to_string()).collect();
        let mut param_defs = HashMap::new();
        param_defs.insert(
            "token1".to_string(),
            ParameterDefinition {
                id: "token1".to_string(),
                description: None,
                default: Some("default1".to_string()),
            },
        );

        // Case 1: No tokens to interpolate
        let empty_tokens = IndexSet::new();
        assert!(!should_prompt_for_parameters(
            &empty_tokens,
            &None,
            false,
            &ParameterMode::None
        ));

        // Case 2: New command (not rerun) without command-line parameters
        assert!(should_prompt_for_parameters(
            &tokens,
            &None,
            false,
            &ParameterMode::None
        ));

        // Case 3: Rerun - should never prompt regardless of parameters
        assert!(!should_prompt_for_parameters(
            &tokens,
            &None,
            true,
            &ParameterMode::None
        ));

        // Case 4: Command-line parameters that cover all tokens
        let mut filled_params = HashMap::new();
        filled_params.insert(
            "token1".to_string(),
            ParameterDefinition {
                id: "token1".to_string(),
                description: None,
                default: Some("value1".to_string()),
            },
        );
        filled_params.insert(
            "token2".to_string(),
            ParameterDefinition {
                id: "token2".to_string(),
                description: None,
                default: Some("value2".to_string()),
            },
        );

        assert!(!should_prompt_for_parameters(
            &tokens,
            &Some(filled_params.clone()),
            false,
            &ParameterMode::Named(vec!["token1=value1".to_string()])
        ));

        // Case 5: Command-line parameters that are missing some tokens
        let mut partial_params = HashMap::new();
        partial_params.insert(
            "token1".to_string(),
            ParameterDefinition {
                id: "token1".to_string(),
                description: None,
                default: Some("value1".to_string()),
            },
        );

        assert!(should_prompt_for_parameters(
            &tokens,
            &Some(partial_params),
            false,
            &ParameterMode::Named(vec!["token1=value1".to_string()])
        ));
    }
}
