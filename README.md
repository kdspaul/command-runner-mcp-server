# Command Runner MCP Server

A Model Context Protocol (MCP) server written in Rust that provides tools for listing directory contents and running git commands, with built-in output transformations.

This could be used to limit the commands that Cursor / Claude Code can run. Sandbox implementations are not standard across tools so if company policy requires that auto-run can only work within sandbox and certain commands do not then auto-run for MCP servers tools can be an easy way to better UX.

The original vision for this MCP server was to have logs stream back to the client but clients still expect the full output in the end so we've switched to stdio.
In my experiments, Rust had the best streaming support so in the future if the MCP clients are able to read and use output streamed to them then we can switch to streaming.

The transformation tools are implemented natively and do not support all the features of the commands that they are emulating.

## Tools

### ls_tool

Lists directory contents using `ls -al`.

**Parameters:**
- `path` (optional): The path to list. Defaults to `.` if not provided.

### git

Run git commands (status, add, commit, checkout).

**Parameters:**
- `subcommand` (required): The git subcommand to run. Must be one of: `status`, `add`, `commit`, `checkout`
- `args` (optional): Array of arguments to pass to the git subcommand

## Common Parameters (All Tools)

All tools support the following optional parameters for output transformation and execution control:

**Output Transformations:**
- `grep_pattern`: Regex pattern to filter output lines (keeps matching lines)
- `invert_grep`: If true, inverts grep to exclude matching lines instead
- `head`: Return only the first N lines of output
- `tail`: Return only the last N lines of output
- `sort`: Sort output lines alphabetically (boolean)
- `unique`: Remove consecutive duplicate lines like `uniq` (boolean)
- `transform_order`: Array specifying custom order of transformations (e.g., `["head", "grep", "sort"]`)

**Execution Control:**
- `timeout_ms`: Command timeout in milliseconds (default: 180000 = 3 minutes)
- `working_dir`: Working directory for command execution
- `env`: Environment variables as `{"KEY": "value"}`

**Default Transformation Order:** grep → sort → unique → head → tail

Only transformations listed in `transform_order` are applied (if specified).

## Examples

List only `.rs` files, sorted:
```json
{"path": "src", "grep_pattern": "\\.rs$", "sort": true}
```

Show only modified files in git status:
```json
{"subcommand": "status", "grep_pattern": "modified:"}
```

Get first 10 lines after filtering:
```json
{"path": "/var/log", "grep_pattern": "error", "head": 10}
```

## Security

### Path Blocking

The server blocks access to specific paths and all their subdirectories. Any attempt to access these paths will return an error.

**Currently blocked paths:**
- `/blocked`
- `/also-blocked`

Path blocking features:
- Blocks both exact path matches and all subdirectories (e.g., `/blocked/subdir`)
- Resolves relative paths and symlinks to prevent bypass attempts
- Handles path traversal attempts (e.g., `/../blocked`)

To modify blocked paths, edit the `BLOCKED_PATHS` constant in `src/security.rs`.

### Shell Injection Protection

The following characters are blocked in all arguments: `; | & $ \` ( ) { } [ ] < > ' " \ * ? ! #`

Use the built-in transformation parameters (grep_pattern, sed_pattern, etc.) instead of shell operators.

### Git Command Restrictions

Only `status`, `add`, `commit`, and `checkout` subcommands are allowed.

## Building

```bash
cargo build
```

For a release build:

```bash
cargo build --release
```

## Running Tests

```bash
cargo test
```

## Manual Testing

### Using the MCP Inspector

```bash
npx @modelcontextprotocol/inspector ./target/debug/command-runner-mcp-server-rust
```

### Using stdin/stdout directly

Initialize the server and list tools:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | ./target/debug/command-runner-mcp-server-rust 2>/dev/null
```

Call the ls_tool:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ls_tool","arguments":{"path":"/tmp"}}}' | ./target/debug/command-runner-mcp-server-rust 2>/dev/null
```

Call ls_tool with transformations:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ls_tool","arguments":{"path":"/tmp","grep_pattern":"\\.log$","head":5}}}' | ./target/debug/command-runner-mcp-server-rust 2>/dev/null
```

## Configuration

To use with Claude Desktop, add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "command-runner": {
      "type": "stdio",
      "command": "/path/to/target/release/command-runner-mcp-server-rust",
      "args": [],
      "env": {}
    }
  }
}
```
