use std::path::Path;

/// Characters that could be used for shell injection
const SHELL_INJECTION_CHARS: &[char] = &[
    ';', '|', '&', '$', '`', '(', ')', '{', '}', '[', ']', '<', '>', '\n', '\r', '\'', '"', '\\',
    '*', '?', '!', '#',
];

/// Human-readable list of forbidden characters for error messages
const SHELL_INJECTION_CHARS_DISPLAY: &str = "; | & $ ` ( ) { } [ ] < > ' \" \\ * ? ! #";

/// Hint about available transformations for error messages
const TRANSFORM_HINT: &str = "Use grep_pattern, sed_pattern, head, tail, sort, or unique parameters to filter/transform output instead of shell operators.";

/// The blocked path prefixes (fictional paths for demonstration)
const BLOCKED_PATHS: &[&str] = &["/blocked", "/also-blocked"];

/// Validation error types
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    ShellInjection(String),
    BlockedPath(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::ShellInjection(arg) => {
                write!(
                    f,
                    "Error: '{}' contains invalid characters. Forbidden characters: {}. {}",
                    arg, SHELL_INJECTION_CHARS_DISPLAY, TRANSFORM_HINT
                )
            }
            ValidationError::BlockedPath(path) => {
                write!(f, "Error: Reading path '{}' is not allowed", path)
            }
        }
    }
}

/// Trait for request types that need validation before execution
pub trait Validatable {
    /// Validate the request, returning an error if invalid
    fn validate(&self) -> Result<(), ValidationError>;
}

/// Check if a string contains shell injection characters
pub fn contains_shell_injection(s: &str) -> bool {
    s.contains(SHELL_INJECTION_CHARS)
}

/// Validate an argument for shell injection
/// Returns the offending argument in the error message
pub fn validate_argument(arg: &str) -> Result<(), ValidationError> {
    if contains_shell_injection(arg) {
        return Err(ValidationError::ShellInjection(arg.to_string()));
    }
    Ok(())
}

/// Resolve a path and check if it matches or is under any blocked path
/// Returns the matched blocked path if found, None otherwise
fn find_blocked_path(path: &str) -> Option<&'static str> {
    // Resolve the path to catch relative path traversal to blocked directories
    let resolved_path = if path.starts_with('/') {
        Path::new(path).to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => return None, // Can't resolve, let the command fail naturally
        }
    };

    // Canonicalize to resolve symlinks and .. components
    let canonical_path = match resolved_path.canonicalize() {
        Ok(p) => p,
        Err(_) => resolved_path, // Path might not exist yet, use as-is
    };

    // Check if path is or is under any blocked path
    let path_str = canonical_path.to_string_lossy();
    for blocked in BLOCKED_PATHS {
        if path_str == *blocked || path_str.starts_with(&format!("{}/", blocked)) {
            return Some(blocked);
        }
    }
    None
}

/// Validate that a path is not blocked
pub fn validate_path(path: &str) -> Result<(), ValidationError> {
    if let Some(blocked) = find_blocked_path(path) {
        return Err(ValidationError::BlockedPath(blocked.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_shell_injection_safe_strings() {
        assert!(!contains_shell_injection("hello"));
        assert!(!contains_shell_injection("path/to/file.txt"));
    }

    #[test]
    fn test_contains_shell_injection_detects_semicolon() {
        assert!(contains_shell_injection("; echo hello"));
    }

    #[test]
    fn test_contains_shell_injection_detects_pipe() {
        assert!(contains_shell_injection("| cat /etc/passwd"));
    }

    #[test]
    fn test_contains_shell_injection_detects_backtick() {
        assert!(contains_shell_injection("`whoami`"));
    }

    #[test]
    fn test_contains_shell_injection_detects_dollar() {
        assert!(contains_shell_injection("$(whoami)"));
    }

    #[test]
    fn test_find_blocked_path_allows_safe_paths() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        assert!(find_blocked_path(temp_dir.path().to_str().unwrap()).is_none());
    }

    #[test]
    fn test_find_blocked_path_blocks_exact() {
        assert_eq!(find_blocked_path("/blocked"), Some("/blocked"));
    }

    #[test]
    fn test_find_blocked_path_blocks_subpath() {
        assert_eq!(find_blocked_path("/blocked/subdir"), Some("/blocked"));
    }

    #[test]
    fn test_find_blocked_path_blocks_also_blocked_exact() {
        assert_eq!(find_blocked_path("/also-blocked"), Some("/also-blocked"));
    }

    #[test]
    fn test_find_blocked_path_blocks_also_blocked_subpath() {
        assert_eq!(find_blocked_path("/also-blocked/subdir"), Some("/also-blocked"));
    }

    #[test]
    fn test_validate_path_returns_error_for_blocked() {
        assert!(matches!(
            validate_path("/blocked"),
            Err(ValidationError::BlockedPath(_))
        ));
    }

    #[test]
    fn test_validate_path_ok_for_allowed() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        assert!(validate_path(temp_dir.path().to_str().unwrap()).is_ok());
    }
}
