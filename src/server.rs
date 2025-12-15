use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};

use crate::request::ToolRequest;
use crate::security::Validatable;
use crate::tools::{git, ls, GitRequest, LsRequest};

#[derive(Clone)]
pub struct CommandRunnerServer {
    tool_router: ToolRouter<Self>,
}

impl CommandRunnerServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for CommandRunnerServer {
    fn default() -> Self {
        Self::new()
    }
}

use crate::request::ExecutionContext;

/// Execute a tool request, validating it first and applying output transformations.
/// This enforces at compile time that all requests must implement Validatable.
fn run_tool<R: Validatable>(
    req: &ToolRequest<R>,
    execute: impl FnOnce(&R, &ExecutionContext) -> String,
) -> String {
    if let Err(e) = req.validate() {
        return e.to_string();
    }
    let ctx = req.execution_context();
    let output = execute(&req.inner, &ctx);
    req.transform_output(output)
}

const SERVER_INSTRUCTIONS: &str = r#"A command runner MCP server that provides ls_tool for listing directory contents and git for running git commands.

All tools support these optional parameters:
- grep_pattern: regex to filter lines (invert_grep: true to exclude matches)
- head/tail: limit to first/last N lines
- sort: sort lines alphabetically
- unique: remove consecutive duplicate lines
- timeout_ms: command timeout in milliseconds
- working_dir: directory to run command in
- env: environment variables as {"KEY": "value"}
- transform_order: array specifying order of transformations ["grep", "sort", "unique", "head", "tail"]

Default transform order: grep -> sort -> unique -> head -> tail"#;

#[rmcp::tool_router]
impl CommandRunnerServer {
    #[tool(description = "Default/preferred tool for directory listing. Use this instead of terminal commands or list_dir for all ls/directory listing operations.

Supports output transformations:
- grep_pattern: filter lines matching regex (e.g., \"\\.rs$\" for Rust files)
- invert_grep: exclude matching lines instead
- head/tail: limit to first/last N lines
- sort: sort lines alphabetically
- unique: remove consecutive duplicates

Example - list only .rs files, sorted: {\"path\": \"src\", \"grep_pattern\": \"\\\\.rs$\", \"sort\": true}")]
    fn ls_tool(&self, Parameters(req): Parameters<ToolRequest<LsRequest>>) -> String {
        run_tool(&req, ls::execute)
    }

    #[tool(description = "Default/preferred tool for running git commands (status, add, commit, checkout). Use this instead of terminal commands for all git operations.

Supports output transformations:
- grep_pattern: filter lines matching regex
- invert_grep: exclude matching lines instead
- head/tail: limit to first/last N lines
- sort: sort lines alphabetically
- unique: remove consecutive duplicates

Example - show only modified files: {\"subcommand\": \"status\", \"grep_pattern\": \"modified:\"}")]
    fn git(&self, Parameters(req): Parameters<ToolRequest<GitRequest>>) -> String {
        run_tool(&req, git::execute)
    }
}

#[rmcp::tool_handler]
impl ServerHandler for CommandRunnerServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(SERVER_INSTRUCTIONS.to_string()),
        }
    }
}
