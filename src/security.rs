use std::path::Path;
use std::sync::LazyLock;

/// Characters that could be used for shell injection
const SHELL_INJECTION_CHARS: &[char] = &[
    ';', '|', '&', '$', '`', '(', ')', '{', '}', '[', ']', '<', '>', '\n', '\r', '\'', '"', '\\',
    '*', '?', '!', '#', '\0', // null byte can truncate strings in some contexts
];

/// Human-readable list of forbidden characters for error messages
const SHELL_INJECTION_CHARS_DISPLAY: &str = "; | & $ ` ( ) { } [ ] < > ' \" \\ * ? ! #";

/// Hint about available transformations for error messages
const TRANSFORM_HINT: &str = "Use grep_pattern, head, tail, sort, or unique parameters to filter/transform output instead of shell operators.";

/// Blocked paths loaded from BLOCKED_PATHS environment variable at startup.
/// Format: semicolon-separated list of absolute paths, e.g., "/etc;/root;/home/user/.ssh"
static BLOCKED_PATHS: LazyLock<Vec<String>> = LazyLock::new(|| {
    std::env::var("BLOCKED_PATHS")
        .unwrap_or_default()
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect()
});

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
    PathTraversal(String),
    RelativeWorkingDir(String),
    DisallowedSubcommand { subcommand: String, allowed: String },
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
            ValidationError::PathTraversal(path) => {
                write!(
                    f,
                    "Error: Path '{}' contains '..', which is not allowed for security reasons.",
                    path
                )
            }
            ValidationError::RelativeWorkingDir(dir) => {
                write!(
                    f,
                    "Error: working_dir '{}' must be an absolute path (starting with '/').",
                    dir
                )
            }
            ValidationError::DisallowedSubcommand { subcommand, allowed } => {
                write!(
                    f,
                    "Error: Subcommand '{}' is not allowed. Allowed subcommands: {}",
                    subcommand, allowed
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

/// Check if a path contains ".." (parent directory traversal)
pub fn contains_traversal(path: &str) -> bool {
    path.contains("..")
}

/// Validate that a path doesn't contain ".." traversal
pub fn validate_no_traversal(path: &str) -> Result<(), ValidationError> {
    if contains_traversal(path) {
        return Err(ValidationError::PathTraversal(path.to_string()));
    }
    Ok(())
}

/// Validate that a path is absolute (starts with '/')
pub fn validate_absolute_path(path: &str) -> Result<(), ValidationError> {
    if !path.starts_with('/') {
        return Err(ValidationError::RelativeWorkingDir(path.to_string()));
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

/// Internal implementation for testability - takes blocked_paths as parameter.
/// Resolves a path and checks if it matches or is under any blocked path.
fn find_blocked_path_impl(path: &str, blocked_paths: &[String]) -> Option<String> {
    // Resolve the path to get absolute path for comparison
    let resolved_path = if path.starts_with('/') {
        Path::new(path).to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => return None, // Can't resolve, let the command fail naturally
        }
    };

    // Try to canonicalize to resolve symlinks (.. is already blocked by validate_no_traversal)
    let canonical_path = match resolved_path.canonicalize() {
        Ok(p) => p,
        Err(_) => resolved_path, // Path might not exist yet, use as-is
    };

    // Check if path is or is under any blocked path
    let path_str = canonical_path.to_string_lossy();
    for blocked in blocked_paths {
        if path_str == *blocked || path_str.starts_with(&format!("{}/", blocked)) {
            return Some(blocked.clone());
        }
    }
    None
}

/// Resolve a path and check if it matches or is under any blocked path.
/// Uses the global BLOCKED_PATHS from environment variable.
fn find_blocked_path(path: &str) -> Option<String> {
    find_blocked_path_impl(path, &BLOCKED_PATHS)
}

/// Validate that a path is not blocked
pub fn validate_path(path: &str) -> Result<(), ValidationError> {
    if let Some(blocked) = find_blocked_path(path) {
        return Err(ValidationError::BlockedPath(blocked));
    }
    Ok(())
}

/// Internal implementation for testability - takes blocked_paths as parameter.
fn validate_path_with_working_dir_impl(path: &str, working_dir: &str, blocked_paths: &[String]) -> Result<(), ValidationError> {
    if !working_dir.starts_with('/') {
        return Err(ValidationError::RelativeWorkingDir(working_dir.to_string()));
    }

    let resolved = if path.starts_with('/') {
        Path::new(path).to_path_buf()
    } else {
        Path::new(working_dir).join(path)
    };

    // Canonicalize to resolve any remaining path components
    let canonical = match resolved.canonicalize() {
        Ok(p) => p,
        Err(_) => resolved, // Path might not exist, use as-is
    };

    let path_str = canonical.to_string_lossy();
    for blocked in blocked_paths {
        if path_str == *blocked || path_str.starts_with(&format!("{}/", blocked)) {
            return Err(ValidationError::BlockedPath(blocked.clone()));
        }
    }
    Ok(())
}

/// Validate that a path resolved against a working directory is not blocked.
/// This handles the case where a relative path combined with working_dir could
/// access a blocked location.
pub fn validate_path_with_working_dir(path: &str, working_dir: &str) -> Result<(), ValidationError> {
    validate_path_with_working_dir_impl(path, working_dir, &BLOCKED_PATHS)
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
        let blocked = vec!["/blocked".to_string()];
        assert_eq!(
            find_blocked_path_impl("/blocked", &blocked),
            Some("/blocked".to_string())
        );
    }

    #[test]
    fn test_find_blocked_path_blocks_subpath() {
        let blocked = vec!["/blocked".to_string()];
        assert_eq!(
            find_blocked_path_impl("/blocked/subdir", &blocked),
            Some("/blocked".to_string())
        );
    }

    #[test]
    fn test_find_blocked_path_blocks_also_blocked_exact() {
        let blocked = vec!["/also-blocked".to_string()];
        assert_eq!(
            find_blocked_path_impl("/also-blocked", &blocked),
            Some("/also-blocked".to_string())
        );
    }

    #[test]
    fn test_find_blocked_path_blocks_also_blocked_subpath() {
        let blocked = vec!["/also-blocked".to_string()];
        assert_eq!(
            find_blocked_path_impl("/also-blocked/subdir", &blocked),
            Some("/also-blocked".to_string())
        );
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

    // Path traversal tests
    #[test]
    fn test_contains_traversal_detects_parent_dir() {
        assert!(contains_traversal("../secret"));
        assert!(contains_traversal("/tmp/../etc"));
        assert!(contains_traversal("foo/bar/../baz"));
    }

    #[test]
    fn test_contains_traversal_allows_safe_paths() {
        assert!(!contains_traversal("/tmp/file"));
        assert!(!contains_traversal("relative/path"));
        assert!(!contains_traversal("."));
    }

    #[test]
    fn test_validate_no_traversal_rejects_parent_dir() {
        assert!(matches!(
            validate_no_traversal("../secret"),
            Err(ValidationError::PathTraversal(_))
        ));
        assert!(matches!(
            validate_no_traversal("/tmp/../etc"),
            Err(ValidationError::PathTraversal(_))
        ));
    }

    #[test]
    fn test_validate_no_traversal_allows_safe_paths() {
        assert!(validate_no_traversal("/tmp/file").is_ok());
        assert!(validate_no_traversal("relative/path").is_ok());
        assert!(validate_no_traversal(".").is_ok());
    }

    // Absolute path tests
    #[test]
    fn test_validate_absolute_path_rejects_relative() {
        assert!(matches!(
            validate_absolute_path("relative/path"),
            Err(ValidationError::RelativeWorkingDir(_))
        ));
        assert!(matches!(
            validate_absolute_path("./current"),
            Err(ValidationError::RelativeWorkingDir(_))
        ));
    }

    #[test]
    fn test_validate_absolute_path_allows_absolute() {
        assert!(validate_absolute_path("/tmp").is_ok());
        assert!(validate_absolute_path("/home/user/dir").is_ok());
    }

    // Blocked path tests using temp directories
    #[test]
    fn test_blocked_path_with_temp_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        // Use canonicalized path (resolves symlinks like /var -> /private/var on macOS)
        let blocked_path = temp_dir.path().canonicalize().unwrap();
        let blocked_path_str = blocked_path.to_string_lossy().to_string();
        let blocked = vec![blocked_path_str.clone()];

        // Exact match should be blocked
        assert_eq!(
            find_blocked_path_impl(&blocked_path_str, &blocked),
            Some(blocked_path_str.clone())
        );
    }

    #[test]
    fn test_blocked_path_subdir_with_temp_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let blocked_path = temp_dir.path().canonicalize().unwrap();
        let blocked_path_str = blocked_path.to_string_lossy().to_string();
        let blocked = vec![blocked_path_str.clone()];

        // Subpath should be blocked (non-existent subpath is resolved relative to parent)
        let subpath = format!("{}/subdir/file.txt", blocked_path_str);
        assert_eq!(
            find_blocked_path_impl(&subpath, &blocked),
            Some(blocked_path_str)
        );
    }

    #[test]
    fn test_not_blocked_when_not_in_list() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let safe_path = temp_dir.path().canonicalize().unwrap();
        let safe_path_str = safe_path.to_string_lossy().to_string();
        let blocked = vec!["/some/other/path".to_string()];

        // Should not be blocked when not in list
        assert!(find_blocked_path_impl(&safe_path_str, &blocked).is_none());
    }

    #[test]
    fn test_blocked_path_with_symlink() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let blocked_path = temp_dir.path().canonicalize().unwrap();
        let blocked_path_str = blocked_path.to_string_lossy().to_string();
        let blocked = vec![blocked_path_str.clone()];

        // Create a symlink to the blocked directory
        let link_dir = tempfile::TempDir::new().unwrap();
        let link_path = link_dir.path().join("link");
        std::os::unix::fs::symlink(temp_dir.path(), &link_path).unwrap();

        // Following symlink should detect blocked path
        let link_path_str = link_path.to_string_lossy().to_string();
        assert_eq!(
            find_blocked_path_impl(&link_path_str, &blocked),
            Some(blocked_path_str)
        );
    }

    // Tests for validate_path_with_working_dir_impl
    #[test]
    fn test_validate_path_with_working_dir_blocks_relative_path_to_blocked() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let blocked_dir = temp_dir.path().join("blocked");
        std::fs::create_dir(&blocked_dir).unwrap();

        let blocked_path = blocked_dir.canonicalize().unwrap();
        let blocked_path_str = blocked_path.to_string_lossy().to_string();
        let blocked = vec![blocked_path_str.clone()];

        let working_dir = temp_dir.path().canonicalize().unwrap();
        let working_dir_str = working_dir.to_string_lossy().to_string();

        // Relative path "blocked" from working_dir should be blocked
        assert!(matches!(
            validate_path_with_working_dir_impl("blocked", &working_dir_str, &blocked),
            Err(ValidationError::BlockedPath(_))
        ));
    }

    #[test]
    fn test_validate_path_with_working_dir_allows_safe_relative_path() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let safe_dir = temp_dir.path().join("safe");
        std::fs::create_dir(&safe_dir).unwrap();

        let blocked = vec!["/some/other/blocked/path".to_string()];

        let working_dir = temp_dir.path().canonicalize().unwrap();
        let working_dir_str = working_dir.to_string_lossy().to_string();

        // Relative path "safe" from working_dir should be allowed
        assert!(validate_path_with_working_dir_impl("safe", &working_dir_str, &blocked).is_ok());
    }

    #[test]
    fn test_validate_path_with_working_dir_rejects_relative_working_dir() {
        let blocked = vec![];
        assert!(matches!(
            validate_path_with_working_dir_impl(".", "relative/dir", &blocked),
            Err(ValidationError::RelativeWorkingDir(_))
        ));
    }

    #[test]
    fn test_validate_path_with_working_dir_absolute_path_ignores_working_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let blocked_dir = temp_dir.path().join("blocked");
        std::fs::create_dir(&blocked_dir).unwrap();

        let blocked_path = blocked_dir.canonicalize().unwrap();
        let blocked_path_str = blocked_path.to_string_lossy().to_string();
        let blocked = vec![blocked_path_str.clone()];

        // Absolute path should be checked directly, ignoring working_dir
        assert!(matches!(
            validate_path_with_working_dir_impl(&blocked_path_str, "/some/other/dir", &blocked),
            Err(ValidationError::BlockedPath(_))
        ));
    }
}
