package main

import (
	"strings"
	"testing"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

func TestFormatResultSuccess(t *testing.T) {
	result := &CommandResult{
		LineCount: 42,
		ExitCode:  0,
	}

	toolResult := FormatResult(result)

	if toolResult.IsError {
		t.Error("expected IsError to be false for exit code 0")
	}

	if len(toolResult.Content) != 1 {
		t.Fatalf("expected 1 content item, got %d", len(toolResult.Content))
	}

	textContent, ok := toolResult.Content[0].(*mcp.TextContent)
	if !ok {
		t.Fatal("expected TextContent")
	}

	if !strings.Contains(textContent.Text, "Lines: 42") {
		t.Errorf("expected 'Lines: 42' in output, got %q", textContent.Text)
	}

	if !strings.Contains(textContent.Text, "Exit code: 0") {
		t.Errorf("expected 'Exit code: 0' in output, got %q", textContent.Text)
	}
}

func TestFormatResultError(t *testing.T) {
	result := &CommandResult{
		LineCount: 10,
		ExitCode:  1,
	}

	toolResult := FormatResult(result)

	if !toolResult.IsError {
		t.Error("expected IsError to be true for non-zero exit code")
	}
}

func TestErrorResult(t *testing.T) {
	toolResult := ErrorResult("something went wrong")

	if !toolResult.IsError {
		t.Error("expected IsError to be true")
	}

	if len(toolResult.Content) != 1 {
		t.Fatalf("expected 1 content item, got %d", len(toolResult.Content))
	}

	textContent, ok := toolResult.Content[0].(*mcp.TextContent)
	if !ok {
		t.Fatal("expected TextContent")
	}

	if textContent.Text != "something went wrong" {
		t.Errorf("expected 'something went wrong', got %q", textContent.Text)
	}
}
