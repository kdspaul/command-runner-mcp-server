use regex::Regex;
use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::security::{Validatable, ValidationError};

/// Execution context extracted from ToolRequest for command execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    pub timeout: Option<Duration>,
    pub working_dir: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

/// Available transformation operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Transformation {
    Grep,
    Sort,
    Unique,
    Head,
    Tail,
}

/// Default transformation order
const DEFAULT_TRANSFORM_ORDER: &[Transformation] = &[
    Transformation::Grep,
    Transformation::Sort,
    Transformation::Unique,
    Transformation::Head,
    Transformation::Tail,
];

/// A wrapper that adds common fields to any tool request.
/// Use `#[serde(flatten)]` on the inner field to merge schemas.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolRequest<T> {
    /// Optional regex pattern to filter output lines (keeps matching lines)
    #[serde(default)]
    pub grep_pattern: Option<String>,

    /// If true, invert grep to exclude matching lines instead of keeping them
    #[serde(default)]
    pub invert_grep: Option<bool>,

    /// Return only the first N lines of output
    #[serde(default)]
    pub head: Option<usize>,

    /// Return only the last N lines of output
    #[serde(default)]
    pub tail: Option<usize>,

    /// Sort output lines alphabetically
    #[serde(default)]
    pub sort: Option<bool>,

    /// Remove duplicate consecutive lines (like uniq)
    #[serde(default)]
    pub unique: Option<bool>,

    /// Timeout in milliseconds for command execution (default: 180000 = 3 minutes)
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Working directory for command execution
    #[serde(default)]
    pub working_dir: Option<String>,

    /// Environment variables to set for command execution
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,

    /// Order to apply transformations. Default: ["grep", "sort", "unique", "head", "tail"]
    /// Only listed transformations will be applied.
    #[serde(default)]
    pub transform_order: Option<Vec<Transformation>>,

    #[serde(flatten)]
    pub inner: T,
}

impl<T: Validatable> Validatable for ToolRequest<T> {
    fn validate(&self) -> Result<(), ValidationError> {
        self.inner.validate()
    }
}

impl<T> ToolRequest<T> {
    /// Default timeout in milliseconds (180 seconds)
    const DEFAULT_TIMEOUT_MS: u64 = 180_000;

    /// Extract execution context for command execution
    pub fn execution_context(&self) -> ExecutionContext {
        let timeout_ms = self.timeout_ms.unwrap_or(Self::DEFAULT_TIMEOUT_MS);
        ExecutionContext {
            timeout: Some(Duration::from_millis(timeout_ms)),
            working_dir: self.working_dir.clone(),
            env: self.env.clone(),
        }
    }

    /// Apply output transformations in the specified order.
    /// Default order: grep -> sort -> unique -> head -> tail
    pub fn transform_output(&self, output: String) -> String {
        let order = self
            .transform_order
            .as_deref()
            .unwrap_or(DEFAULT_TRANSFORM_ORDER);

        let mut result = output;

        for transform in order {
            result = match transform {
                Transformation::Grep => self.apply_grep(result),
                Transformation::Sort => self.apply_sort(result),
                Transformation::Unique => self.apply_unique(result),
                Transformation::Head => self.apply_head(result),
                Transformation::Tail => self.apply_tail(result),
            };

            // Stop on error
            if result.starts_with("Error:") {
                return result;
            }
        }

        result
    }

    fn apply_grep(&self, output: String) -> String {
        match &self.grep_pattern {
            Some(pattern) => {
                let regex = match Regex::new(pattern) {
                    Ok(r) => r,
                    Err(e) => return format!("Error: Invalid grep pattern: {}", e),
                };
                let invert = self.invert_grep.unwrap_or(false);
                output
                    .lines()
                    .filter(|line| {
                        let matches = regex.is_match(line);
                        if invert { !matches } else { matches }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            None => output,
        }
    }

    fn apply_sort(&self, output: String) -> String {
        if self.sort.unwrap_or(false) {
            let mut lines: Vec<&str> = output.lines().collect();
            lines.sort();
            lines.join("\n")
        } else {
            output
        }
    }

    fn apply_unique(&self, output: String) -> String {
        if self.unique.unwrap_or(false) {
            let mut result = Vec::new();
            let mut prev: Option<&str> = None;
            for line in output.lines() {
                if prev != Some(line) {
                    result.push(line);
                    prev = Some(line);
                }
            }
            result.join("\n")
        } else {
            output
        }
    }

    fn apply_head(&self, output: String) -> String {
        match self.head {
            Some(n) => {
                output.lines().take(n).collect::<Vec<_>>().join("\n")
            }
            None => output,
        }
    }

    fn apply_tail(&self, output: String) -> String {
        match self.tail {
            Some(n) => {
                let lines: Vec<&str> = output.lines().collect();
                let total = lines.len();
                if n >= total {
                    output
                } else {
                    lines.into_iter().skip(total - n).collect::<Vec<_>>().join("\n")
                }
            }
            None => output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::LsRequest;

    fn make_request(
        grep_pattern: Option<&str>,
        invert_grep: Option<bool>,
        head: Option<usize>,
        tail: Option<usize>,
        sort: Option<bool>,
        unique: Option<bool>,
    ) -> ToolRequest<LsRequest> {
        make_request_with_order(grep_pattern, invert_grep, head, tail, sort, unique, None)
    }

    fn make_request_with_order(
        grep_pattern: Option<&str>,
        invert_grep: Option<bool>,
        head: Option<usize>,
        tail: Option<usize>,
        sort: Option<bool>,
        unique: Option<bool>,
        transform_order: Option<Vec<Transformation>>,
    ) -> ToolRequest<LsRequest> {
        ToolRequest {
            grep_pattern: grep_pattern.map(String::from),
            invert_grep,
            head,
            tail,
            sort,
            unique,
            timeout_ms: None,
            working_dir: None,
            env: None,
            transform_order,
            inner: LsRequest {
                path: ".".to_string(),
            },
        }
    }

    // Grep tests
    #[test]
    fn test_grep_no_pattern() {
        let req = make_request(None, None, None, None, None, None);
        let output = "line1\nline2\nline3".to_string();
        assert_eq!(req.transform_output(output.clone()), output);
    }

    #[test]
    fn test_grep_with_pattern() {
        let req = make_request(Some("2"), None, None, None, None, None);
        let output = "line1\nline2\nline3".to_string();
        assert_eq!(req.transform_output(output), "line2");
    }

    #[test]
    fn test_grep_regex_pattern() {
        let req = make_request(Some(r"line[13]"), None, None, None, None, None);
        let output = "line1\nline2\nline3".to_string();
        assert_eq!(req.transform_output(output), "line1\nline3");
    }

    #[test]
    fn test_grep_invalid_regex() {
        let req = make_request(Some("[invalid"), None, None, None, None, None);
        let output = "line1\nline2".to_string();
        let result = req.transform_output(output);
        assert!(result.contains("Invalid grep pattern"));
    }

    #[test]
    fn test_grep_inverted() {
        let req = make_request(Some("2"), Some(true), None, None, None, None);
        let output = "line1\nline2\nline3".to_string();
        assert_eq!(req.transform_output(output), "line1\nline3");
    }

    // Head/tail tests
    #[test]
    fn test_head() {
        let req = make_request(None, None, Some(2), None, None, None);
        let output = "line1\nline2\nline3\nline4".to_string();
        assert_eq!(req.transform_output(output), "line1\nline2");
    }

    #[test]
    fn test_tail() {
        let req = make_request(None, None, None, Some(2), None, None);
        let output = "line1\nline2\nline3\nline4".to_string();
        assert_eq!(req.transform_output(output), "line3\nline4");
    }

    #[test]
    fn test_head_and_tail() {
        let req = make_request(None, None, Some(3), Some(2), None, None);
        let output = "line1\nline2\nline3\nline4".to_string();
        assert_eq!(req.transform_output(output), "line2\nline3");
    }

    // Sort tests
    #[test]
    fn test_sort() {
        let req = make_request(None, None, None, None, Some(true), None);
        let output = "cherry\napple\nbanana".to_string();
        assert_eq!(req.transform_output(output), "apple\nbanana\ncherry");
    }

    // Unique tests
    #[test]
    fn test_unique() {
        let req = make_request(None, None, None, None, None, Some(true));
        let output = "a\na\nb\nb\nb\na".to_string();
        assert_eq!(req.transform_output(output), "a\nb\na");
    }

    #[test]
    fn test_sort_then_unique() {
        let req = make_request(None, None, None, None, Some(true), Some(true));
        let output = "b\na\nb\na\nc".to_string();
        assert_eq!(req.transform_output(output), "a\nb\nc");
    }

    // Combination tests
    #[test]
    fn test_grep_then_head() {
        let req = make_request(Some("line"), None, Some(2), None, None, None);
        let output = "line1\nother\nline2\nline3".to_string();
        assert_eq!(req.transform_output(output), "line1\nline2");
    }

    // Deserialization tests
    #[test]
    fn test_deserialize_with_all_fields() {
        let json = r#"{
            "path": "/tmp",
            "grep_pattern": "test",
            "invert_grep": true,
            "head": 10,
            "tail": 5,
            "sort": true,
            "unique": true,
            "timeout_ms": 5000,
            "working_dir": "/home",
            "env": {"FOO": "bar"}
        }"#;
        let req: ToolRequest<LsRequest> = serde_json::from_str(json).unwrap();
        assert_eq!(req.inner.path, "/tmp");
        assert_eq!(req.grep_pattern, Some("test".to_string()));
        assert_eq!(req.invert_grep, Some(true));
        assert_eq!(req.head, Some(10));
        assert_eq!(req.tail, Some(5));
        assert_eq!(req.sort, Some(true));
        assert_eq!(req.unique, Some(true));
        assert_eq!(req.timeout_ms, Some(5000));
        assert_eq!(req.working_dir, Some("/home".to_string()));
        assert_eq!(req.env.as_ref().unwrap().get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_deserialize_minimal() {
        let json = r#"{"path": "/tmp"}"#;
        let req: ToolRequest<LsRequest> = serde_json::from_str(json).unwrap();
        assert_eq!(req.inner.path, "/tmp");
        assert_eq!(req.grep_pattern, None);
        assert_eq!(req.head, None);
    }

    // Transform order tests
    #[test]
    fn test_custom_transform_order_head_before_grep() {
        // Default order: grep first, then head -> "line1\nline2" (2 lines matching "line")
        // Custom order: head first, then grep -> "line1" (head 2, then grep for "1")
        let req = make_request_with_order(
            Some("1"),
            None,
            Some(2),
            None,
            None,
            None,
            Some(vec![Transformation::Head, Transformation::Grep]),
        );
        let output = "line1\nline2\nline3\nline4".to_string();
        assert_eq!(req.transform_output(output), "line1");
    }

    #[test]
    fn test_custom_transform_order_tail_before_sort() {
        // Get last 3, then sort them
        let req = make_request_with_order(
            None,
            None,
            None,
            Some(3),
            Some(true),
            None,
            Some(vec![Transformation::Tail, Transformation::Sort]),
        );
        let output = "delta\ncharlie\nbravo\nalpha".to_string();
        // tail 3 = charlie, bravo, alpha; sorted = alpha, bravo, charlie
        assert_eq!(req.transform_output(output), "alpha\nbravo\ncharlie");
    }

    #[test]
    fn test_custom_transform_order_only_some_transforms() {
        // Only apply head, skip everything else even if set
        let req = make_request_with_order(
            Some("1"),  // This grep pattern is set but won't be applied
            None,
            Some(2),
            None,
            Some(true),  // Sort is set but won't be applied
            None,
            Some(vec![Transformation::Head]),  // Only head is in the order
        );
        let output = "line3\nline1\nline2".to_string();
        // Only head is applied, so we get first 2 lines unsorted
        assert_eq!(req.transform_output(output), "line3\nline1");
    }

    #[test]
    fn test_deserialize_transform_order() {
        let json = r#"{
            "path": "/tmp",
            "head": 5,
            "grep_pattern": "test",
            "transform_order": ["head", "grep", "sort"]
        }"#;
        let req: ToolRequest<LsRequest> = serde_json::from_str(json).unwrap();
        assert_eq!(req.transform_order, Some(vec![
            Transformation::Head,
            Transformation::Grep,
            Transformation::Sort,
        ]));
    }
}
