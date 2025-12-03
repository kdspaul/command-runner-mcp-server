package main

import (
	"fmt"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

// FormatResult creates a result with execution metadata
func FormatResult(result *CommandResult) *mcp.CallToolResult {
	msg := fmt.Sprintf("Lines: %d\nExit code: %d",
		result.LineCount, result.ExitCode)

	return &mcp.CallToolResult{
		Content: []mcp.Content{
			&mcp.TextContent{Text: msg},
		},
		IsError: result.ExitCode != 0,
	}
}

// ErrorResult creates an error result
func ErrorResult(msg string) *mcp.CallToolResult {
	return &mcp.CallToolResult{
		Content: []mcp.Content{
			&mcp.TextContent{Text: msg},
		},
		IsError: true,
	}
}
