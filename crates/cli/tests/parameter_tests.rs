#[cfg(test)]
mod tests {
    use indexmap::IndexSet;
    use rust_cuts_cli::arguments::style::{determine, Style};
    use rust_cuts_cli::arguments::validation::should_prompt_for_parameters;
    use rust_cuts_core::command_definitions::ParameterDefinition;
    use std::collections::HashMap;

    #[test]
    fn test_parameter_mode_determination() {
        // Test with no parameters
        let empty_named: Vec<String> = vec![];
        let empty_positional: Vec<String> = vec![];
        let style = determine(&empty_named, &empty_positional).unwrap();
        assert!(matches!(style, Style::None));

        // Test with named parameters
        let named = vec!["key1=value1".to_string(), "key2=value2".to_string()];
        let style = determine(&named, &empty_positional).unwrap();
        assert!(matches!(style, Style::Named(_)));

        // Test with positional arguments
        let positional = vec!["value1".to_string(), "value2".to_string()];
        let style = determine(&empty_named, &positional).unwrap();
        assert!(matches!(style, Style::Positional(_)));

        // Test mixed parameters (should error)
        let result = determine(&named, &positional);
        assert!(result.is_err());
    }

    #[test]
    fn test_should_prompt_for_parameters() {
        let tokens: IndexSet<String> = ["token1", "token2"]
            .iter()
            .map(ToString::to_string)
            .collect();
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
        assert!(!should_prompt_for_parameters::<std::hash::RandomState>(
            &empty_tokens,
            None,
            false,
            &Style::None
        ));

        // Case 2: New command (not rerun) without command-line parameters
        assert!(should_prompt_for_parameters::<std::hash::RandomState>(
            &tokens,
            None,
            false,
            &Style::None
        ));

        // Case 3: Rerun - should never prompt regardless of parameters
        assert!(!should_prompt_for_parameters::<std::hash::RandomState>(
            &tokens,
            None,
            true,
            &Style::None
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
            Some(&filled_params),
            false,
            &Style::Named(vec!["token1=value1".to_string()])
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
            Some(&partial_params),
            false,
            &Style::Named(vec!["token1=value1".to_string()])
        ));
    }
}
