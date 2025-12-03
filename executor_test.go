package main

import (
	"context"
	"strings"
	"testing"
	"time"
)

func TestStreamCommandEcho(t *testing.T) {
	ctx := context.Background()

	result, err := StreamCommand(ctx, nil, "echo", "hello", "world")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d", result.ExitCode)
	}

	if result.LineCount != 1 {
		t.Errorf("expected 1 line, got %d", result.LineCount)
	}
}

func TestStreamCommandMultipleLines(t *testing.T) {
	ctx := context.Background()

	// Use printf to output multiple lines
	result, err := StreamCommand(ctx, nil, "printf", "line1\nline2\nline3\n")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d", result.ExitCode)
	}

	if result.LineCount != 3 {
		t.Errorf("expected 3 lines, got %d", result.LineCount)
	}
}

func TestStreamCommandNonZeroExit(t *testing.T) {
	ctx := context.Background()

	result, err := StreamCommand(ctx, nil, "sh", "-c", "exit 42")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ExitCode != 42 {
		t.Errorf("expected exit code 42, got %d", result.ExitCode)
	}
}

func TestStreamCommandNotFound(t *testing.T) {
	ctx := context.Background()

	_, err := StreamCommand(ctx, nil, "nonexistent-command-12345")
	if err == nil {
		t.Error("expected error for non-existent command")
	}
}

func TestStreamCommandWithTimeout(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 100*time.Millisecond)
	defer cancel()

	result, err := StreamCommand(ctx, nil, "sleep", "10")
	// Either returns an error or a non-zero exit code due to being killed
	if err == nil && result.ExitCode == 0 {
		t.Error("expected timeout to kill the command")
	}
}

func TestStreamCommandStderr(t *testing.T) {
	ctx := context.Background()

	// Write to stderr
	result, err := StreamCommand(ctx, nil, "sh", "-c", "echo error >&2")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d", result.ExitCode)
	}

	// stderr is also counted as lines
	if result.LineCount != 1 {
		t.Errorf("expected 1 line from stderr, got %d", result.LineCount)
	}
}

func TestStreamPipeBasic(t *testing.T) {
	reader := strings.NewReader("line1\nline2\nline3\n")

	lineCount := StreamPipe(context.Background(), nil, reader, 0)

	if lineCount != 3 {
		t.Errorf("expected 3 lines, got %d", lineCount)
	}
}

func TestStreamPipeEmpty(t *testing.T) {
	reader := strings.NewReader("")

	lineCount := StreamPipe(context.Background(), nil, reader, 0)

	if lineCount != 0 {
		t.Errorf("expected 0 lines, got %d", lineCount)
	}
}

func TestStreamPipeContinuesCount(t *testing.T) {
	reader := strings.NewReader("line1\nline2\n")

	// Start from line 5
	lineCount := StreamPipe(context.Background(), nil, reader, 5)

	if lineCount != 7 {
		t.Errorf("expected 7 (5+2), got %d", lineCount)
	}
}
