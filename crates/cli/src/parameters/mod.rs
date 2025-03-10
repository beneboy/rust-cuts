// Export public items from submodules
pub mod mode;
pub mod validation;
pub mod processing;

// Re-exports for convenience
pub use mode::ParameterMode;
pub use mode::ParameterModeProvider;
pub use processing::process_command_line_parameters;
pub use mode::determine_parameter_mode;