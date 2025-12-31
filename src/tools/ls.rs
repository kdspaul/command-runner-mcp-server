use rmcp::schemars;
use serde::Deserialize;
use std::process::Command;

use crate::executor::run_command;
use crate::request::ExecutionContext;
use crate::security::{validate_argument, validate_no_traversal, validate_not_flag, validate_path, validate_path_with_working_dir, Validatable, ValidationError};

/// Request parameters for the ls tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LsRequest {
    /// The path to list contents of. Defaults to "." if not provided.
    #[serde(default = "default_path")]
    pub path: String,
}

fn default_path() -> String {
    ".".to_string()
}

impl Validatable for LsRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_argument(&self.path)?;
        validate_not_flag(&self.path)?;
        // Block ".." to keep code simple and prevent any attempts to access blocked paths
        validate_no_traversal(&self.path)?;
        validate_path(&self.path)?;
        Ok(())
    }
}

/// Execute the ls command with a validated request and execution context
pub fn execute(req: &LsRequest, ctx: &ExecutionContext) -> String {
    // Validate that path combined with working_dir doesn't access blocked paths
    if let Some(ref working_dir) = ctx.working_dir {
        if let Err(e) = validate_path_with_working_dir(&req.path, working_dir) {
            return format!("Error: {}", e);
        }
    }

    let mut cmd = Command::new("ls");
    cmd.args(["-al", &req.path]);
    run_command(cmd, ctx).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let _ = writeln!(
            File::create(temp_dir.path().join("file1.txt")).unwrap(),
            "content1"
        );
        let _ = writeln!(
            File::create(temp_dir.path().join("file2.rs")).unwrap(),
            "fn main() {{}}"
        );
        let _ = writeln!(
            File::create(temp_dir.path().join(".hidden")).unwrap(),
            "secret"
        );
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        let _ = writeln!(
            File::create(temp_dir.path().join("subdir/nested.txt")).unwrap(),
            "nested"
        );
        temp_dir
    }

    #[test]
    fn test_ls_tool_lists_files() {
        let temp_dir = setup_test_dir();
        let req = LsRequest {
            path: temp_dir.path().to_string_lossy().to_string(),
        };
        assert!(req.validate().is_ok());
        let result = execute(&req, &ExecutionContext::default());
        assert!(result.contains("file1.txt"));
        assert!(result.contains("file2.rs"));
        assert!(result.contains("subdir"));
    }

    #[test]
    fn test_ls_tool_shows_hidden_files() {
        let temp_dir = setup_test_dir();
        let req = LsRequest {
            path: temp_dir.path().to_string_lossy().to_string(),
        };
        assert!(req.validate().is_ok());
        let result = execute(&req, &ExecutionContext::default());
        assert!(result.contains(".hidden"));
    }

    #[test]
    fn test_ls_tool_nonexistent_path() {
        let req = LsRequest {
            path: "/nonexistent/path".to_string(),
        };
        assert!(req.validate().is_ok());
        let result = execute(&req, &ExecutionContext::default());
        assert!(result.contains("No such file or directory"));
    }

    #[test]
    fn test_ls_request_default() {
        let request: LsRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(request.path, ".");
    }

    #[test]
    fn test_validate_blocks_shell_injection_semicolon() {
        let req = LsRequest {
            path: "/tmp; echo hello".to_string(),
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::ShellInjection(_))
        ));
    }

    #[test]
    fn test_validate_blocks_shell_injection_pipe() {
        let req = LsRequest {
            path: "/tmp | echo hello".to_string(),
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::ShellInjection(_))
        ));
    }

    #[test]
    fn test_validate_blocks_shell_injection_backtick() {
        let req = LsRequest {
            path: "`echo hello`".to_string(),
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::ShellInjection(_))
        ));
    }

    #[test]
    fn test_validate_blocks_shell_injection_dollar() {
        let req = LsRequest {
            path: "$(echo hello)".to_string(),
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::ShellInjection(_))
        ));
    }

    #[test]
    fn test_validate_allows_other_paths() {
        let req = LsRequest {
            path: "/tmp".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_validate_blocks_flag_injection_single_dash() {
        let req = LsRequest {
            path: "-la".to_string(),
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::FlagInjection(_))
        ));
    }

    #[test]
    fn test_validate_blocks_flag_injection_double_dash() {
        let req = LsRequest {
            path: "--help".to_string(),
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::FlagInjection(_))
        ));
    }

    #[test]
    fn test_validate_allows_paths_with_internal_dashes() {
        let req = LsRequest {
            path: "/path/with-dash/file".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_validate_blocks_path_traversal() {
        let req = LsRequest {
            path: "/tmp/../etc".to_string(),
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::PathTraversal(_))
        ));
    }

    #[test]
    fn test_validate_blocks_path_traversal_relative() {
        let req = LsRequest {
            path: "../secret".to_string(),
        };
        assert!(matches!(
            req.validate(),
            Err(ValidationError::PathTraversal(_))
        ));
    }
}
