package main

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"os/exec"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

// CommandResult contains the result of a command execution
type CommandResult struct {
	LineCount int // Number of lines streamed
	ExitCode  int // Command exit code
}

// StreamCommand executes a command and streams output via progress notifications.
func StreamCommand(ctx context.Context, req *mcp.CallToolRequest, command string, args ...string) (*CommandResult, error) {
	cmd := exec.CommandContext(ctx, command, args...)

	// Get stdout pipe for streaming
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	// Capture stderr separately
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return nil, fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("failed to start command: %w", err)
	}

	result := &CommandResult{}

	// Stream stdout
	result.LineCount = StreamPipe(ctx, req, stdout, 0)

	// Stream stderr
	result.LineCount = StreamPipe(ctx, req, stderr, result.LineCount)

	// Wait for command to complete
	if err := cmd.Wait(); err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			result.ExitCode = exitErr.ExitCode()
		} else {
			return nil, fmt.Errorf("command failed: %w", err)
		}
	}

	return result, nil
}

// StreamPipe reads from a pipe and sends progress notifications for each line
func StreamPipe(ctx context.Context, req *mcp.CallToolRequest, pipe io.Reader, lineNum int) int {
	scanner := bufio.NewScanner(pipe)

	for scanner.Scan() {
		line := scanner.Text()
		lineNum++

		// Send progress notification if token provided
		if req != nil {
			if token := req.Params.GetProgressToken(); token != nil {
				params := &mcp.ProgressNotificationParams{
					ProgressToken: token,
					Progress:      float64(lineNum),
					Message:       line,
				}
				req.Session.NotifyProgress(ctx, params)
			}
		}
	}

	return lineNum
}
