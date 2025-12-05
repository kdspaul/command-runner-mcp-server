# command-runner-mcp-server

An MCP (Model Context Protocol) server that provides tools for running common shell commands.

## Tools

| Tool | Description |
|------|-------------|
| `cat` | Read and output file contents |
| `ls` | List directory contents |
| `bazel` | Run bazel build or test commands |

## Building

```bash
# Build for current platform
make build

# Build for all platforms (linux/darwin, amd64/arm64)
make build-all-platforms

# Build for a specific platform
make build-linux-amd64
make build-linux-arm64
make build-darwin-amd64
make build-darwin-arm64
```

Binaries are output to `dist/<platform>-<arch>/command-runner-mcp-server`.

## Testing

```bash
# Run all tests
make test

# Run tests with coverage
make test-coverage
```

## Testing with MCP Inspector

[MCP Inspector](https://github.com/modelcontextprotocol/inspector) is a debugging tool for MCP servers.

1. Build the server:
   ```bash
   make build
   ```

2. Run the inspector:
   ```bash
   npx @modelcontextprotocol/inspector dist/darwin-arm64/command-runner-mcp-server
   ```

3. Open the URL shown in the terminal (typically `http://localhost:6274`)

4. Click "Connect" to connect to the server

5. Navigate to "Tools" to see and test `cat`, `ls`, and `bazel`

## Usage with MCP Clients

Add the server to your MCP client configuration:

```json
{
  "mcpServers": {
    "command-runner": {
      "command": "/path/to/dist/darwin-arm64/command-runner-mcp-server"
    }
  }
}
```

### Config file locations

- **Claude Desktop (macOS):** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Claude Code:** `.claude/settings.json` or `~/.claude/settings.json`

## Other Make Targets

| Target | Description |
|--------|-------------|
| `make tidy` | Run `go mod tidy` |
| `make fmt` | Format Go files |
| `make vet` | Run static analysis |
| `make run` | Build and run the server |
| `make clean` | Remove build artifacts |
