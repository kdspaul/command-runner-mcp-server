use rmcp::schemars;
use serde::Deserialize;
use std::process::Command;

use crate::executor::run_command;
use crate::request::ExecutionContext;
use crate::security::{validate_argument, Validatable, ValidationError};

/// Allowed git subcommands
const ALLOWED_GIT_SUBCOMMANDS: &[&str] = &["status", "add", "commit", "checkout"];

/// Request parameters for the git tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GitRequest {
    /// The git subcommand to run (status, add, commit, checkout)
    pub subcommand: String,
    /// Arguments to pass to the git subcommand
    #[serde(default)]
    pub args: Vec<String>,
}

impl Validatable for GitRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate subcommand is allowed
        if !ALLOWED_GIT_SUBCOMMANDS.contains(&self.subcommand.as_str()) {
            return Err(ValidationError::ShellInjection(format!(
                "Subcommand '{}' is not allowed. Allowed subcommands: {}",
                self.subcommand,
                ALLOWED_GIT_SUBCOMMANDS.join(", ")
            )));
        }

        // Check for shell injection in subcommand
        validate_argument(&self.subcommand)?;

        // Check for shell injection in arguments
        for arg in &self.args {
            validate_argument(arg)?;
        }

        Ok(())
    }
}

/// Execute a git command with a validated request and execution context
pub fn execute(req: &GitRequest, ctx: &ExecutionContext) -> String {
    let mut cmd = Command::new("git");
    cmd.arg(&req.subcommand);
    cmd.args(&req.args);
    run_command(cmd, ctx).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    #[test]
    fn test_git_status() {
        let temp_dir = TempDir::new().unwrap();
        // Initialize a git repo
        StdCommand::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let req = GitRequest {
            subcommand: "status".to_string(),
            args: vec![],
        };
        assert!(req.validate().is_ok());

        // Use the temp dir as working directory
        let ctx = ExecutionContext {
            working_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            ..Default::default()
        };
        let result = execute(&req, &ctx);
        assert!(!result.starts_with("Error: Failed to execute"));
    }

    #[test]
    fn test_validate_rejects_disallowed_subcommand() {
        let req = GitRequest {
            subcommand: "push".to_string(),
            args: vec![],
        };
        let err = req.validate().unwrap_err();
        assert!(err.to_string().contains("not allowed"));
    }

    #[test]
    fn test_validate_rejects_shell_injection_in_subcommand() {
        let req = GitRequest {
            subcommand: "status; echo hello".to_string(),
            args: vec![],
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_shell_injection_in_args() {
        let req = GitRequest {
            subcommand: "status".to_string(),
            args: vec!["; echo hello".to_string()],
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::ShellInjection(_))
        ));
    }

    #[test]
    fn test_validate_rejects_shell_injection_pipe_in_args() {
        let req = GitRequest {
            subcommand: "add".to_string(),
            args: vec!["file.txt | cat /etc/passwd".to_string()],
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::ShellInjection(_))
        ));
    }
}
