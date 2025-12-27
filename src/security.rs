use std::path::Path;

/// Characters that could be used for shell injection
const SHELL_INJECTION_CHARS: &[char] = &[
    ';', '|', '&', '$', '`', '(', ')', '{', '}', '[', ']', '<', '>', '\n', '\r', '\'', '"', '\\',
    '*', '?', '!', '#', '\0', // null byte can truncate strings in some contexts
];

/// Human-readable list of forbidden characters for error messages
const SHELL_INJECTION_CHARS_DISPLAY: &str = "; | & $ ` ( ) { } [ ] < > ' \" \\ * ? ! #";

/// Hint about available transformations for error messages
const TRANSFORM_HINT: &str = "Use grep_pattern, sed_pattern, head, tail, sort, or unique parameters to filter/transform output instead of shell operators.";

/// The blocked path prefixes (fictional paths for demonstration)
const BLOCKED_PATHS: &[&str] = &["/blocked", "/also-blocked"];

/// Environment variable names that could be used for code injection or privilege escalation
const DANGEROUS_ENV_VARS: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "PATH",
    "HOME",
    "USER",
    "SHELL",
    "IFS",
    "BASH_ENV",
    "ENV",
    "CDPATH",
    "GLOBIGNORE",
    "BASH_FUNC_",
    "PS1",
    "PS2",
    "PS4",
    "PROMPT_COMMAND",
];

/// Validation error types
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    ShellInjection(String),
    BlockedPath(String),
    FlagInjection(String),
    DangerousEnvVar(String),
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
            ValidationError::FlagInjection(arg) => {
                write!(
                    f,
                    "Error: '{}' looks like a flag (starts with '-'). Arguments cannot start with '-' to prevent flag injection.",
                    arg
                )
            }
            ValidationError::DangerousEnvVar(var) => {
                write!(
                    f,
                    "Error: Setting environment variable '{}' is not allowed for security reasons.",
                    var
                )
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

/// Check if a string looks like a command-line flag (starts with -)
pub fn is_flag_like(s: &str) -> bool {
    s.starts_with('-') && s != "-" && s != "--"
}

/// Validate that an argument doesn't look like a flag
/// Use this for positional arguments that shouldn't be flags
pub fn validate_not_flag(arg: &str) -> Result<(), ValidationError> {
    if is_flag_like(arg) {
        return Err(ValidationError::FlagInjection(arg.to_string()));
    }
    Ok(())
}

/// Check if an environment variable name is dangerous
fn is_dangerous_env_var(name: &str) -> bool {
    let upper = name.to_uppercase();
    DANGEROUS_ENV_VARS
        .iter()
        .any(|&dangerous| upper == dangerous || upper.starts_with(dangerous))
}

/// Validate that an environment variable is safe to set
pub fn validate_env_var(name: &str, value: &str) -> Result<(), ValidationError> {
    // Check for dangerous variable names
    if is_dangerous_env_var(name) {
        return Err(ValidationError::DangerousEnvVar(name.to_string()));
    }
    // Check for shell injection in both name and value
    if contains_shell_injection(name) {
        return Err(ValidationError::ShellInjection(name.to_string()));
    }
    if contains_shell_injection(value) {
        return Err(ValidationError::ShellInjection(value.to_string()));
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

    // Null byte detection
    #[test]
    fn test_contains_shell_injection_detects_null_byte() {
        assert!(contains_shell_injection("file\0.txt"));
    }

    // Flag injection tests
    #[test]
    fn test_is_flag_like_detects_single_dash() {
        assert!(is_flag_like("-a"));
        assert!(is_flag_like("-verbose"));
    }

    #[test]
    fn test_is_flag_like_detects_double_dash() {
        assert!(is_flag_like("--help"));
        assert!(is_flag_like("--version"));
    }

    #[test]
    fn test_is_flag_like_allows_bare_dashes() {
        // Single dash (stdin) and double dash (end of options) are allowed
        assert!(!is_flag_like("-"));
        assert!(!is_flag_like("--"));
    }

    #[test]
    fn test_is_flag_like_allows_normal_paths() {
        assert!(!is_flag_like("file.txt"));
        assert!(!is_flag_like("/path/to/file"));
        assert!(!is_flag_like("path/with-dash/file"));
    }

    #[test]
    fn test_validate_not_flag_rejects_flags() {
        assert!(matches!(
            validate_not_flag("--help"),
            Err(ValidationError::FlagInjection(_))
        ));
        assert!(matches!(
            validate_not_flag("-rf"),
            Err(ValidationError::FlagInjection(_))
        ));
    }

    #[test]
    fn test_validate_not_flag_allows_normal_args() {
        assert!(validate_not_flag("file.txt").is_ok());
        assert!(validate_not_flag("/path/to/file").is_ok());
        assert!(validate_not_flag(".").is_ok());
    }

    // Dangerous env var tests
    #[test]
    fn test_is_dangerous_env_var_blocks_ld_preload() {
        assert!(is_dangerous_env_var("LD_PRELOAD"));
        assert!(is_dangerous_env_var("ld_preload")); // case insensitive
    }

    #[test]
    fn test_is_dangerous_env_var_blocks_path() {
        assert!(is_dangerous_env_var("PATH"));
    }

    #[test]
    fn test_is_dangerous_env_var_blocks_dyld() {
        assert!(is_dangerous_env_var("DYLD_INSERT_LIBRARIES"));
        assert!(is_dangerous_env_var("DYLD_LIBRARY_PATH"));
    }

    #[test]
    fn test_is_dangerous_env_var_blocks_bash_func_prefix() {
        assert!(is_dangerous_env_var("BASH_FUNC_foo"));
    }

    #[test]
    fn test_is_dangerous_env_var_allows_safe_vars() {
        assert!(!is_dangerous_env_var("MY_VAR"));
        assert!(!is_dangerous_env_var("FOO"));
        assert!(!is_dangerous_env_var("DEBUG"));
    }

    #[test]
    fn test_validate_env_var_rejects_dangerous_names() {
        assert!(matches!(
            validate_env_var("LD_PRELOAD", "/evil/lib.so"),
            Err(ValidationError::DangerousEnvVar(_))
        ));
        assert!(matches!(
            validate_env_var("PATH", "/evil/bin"),
            Err(ValidationError::DangerousEnvVar(_))
        ));
    }

    #[test]
    fn test_validate_env_var_rejects_shell_injection_in_name() {
        assert!(matches!(
            validate_env_var("VAR;rm", "value"),
            Err(ValidationError::ShellInjection(_))
        ));
    }

    #[test]
    fn test_validate_env_var_rejects_shell_injection_in_value() {
        assert!(matches!(
            validate_env_var("MY_VAR", "$(whoami)"),
            Err(ValidationError::ShellInjection(_))
        ));
    }

    #[test]
    fn test_validate_env_var_allows_safe_vars() {
        assert!(validate_env_var("MY_VAR", "safe_value").is_ok());
        assert!(validate_env_var("DEBUG", "true").is_ok());
    }
}
