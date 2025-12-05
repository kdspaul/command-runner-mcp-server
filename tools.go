package main

import (
	"context"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

// CatInput defines the input for the cat tool
type CatInput struct {
	Path string `json:"path" jsonschema_description:"Path to the file to read"`
}

// RegisterCatTool registers the cat tool with the server
func RegisterCatTool(server *mcp.Server) {
	tool := &mcp.Tool{
		Name:        "cat",
		Description: "Read and output file contents",
	}

	handler := func(ctx context.Context, req *mcp.CallToolRequest, input CatInput) (*mcp.CallToolResult, CatInput, error) {
		if input.Path == "" {
			return ErrorResult("path is required"), input, nil
		}

		result, err := StreamCommand(ctx, req, "cat", input.Path)
		if err != nil {
			return ErrorResult(err.Error()), input, nil
		}

		return FormatResult(result), input, nil
	}

	mcp.AddTool(server, tool, handler)
}

// LsInput defines the input for the ls tool
type LsInput struct {
	Path string `json:"path" jsonschema_description:"Path to the directory to list"`
}

// RegisterLsTool registers the ls tool with the server
func RegisterLsTool(server *mcp.Server) {
	tool := &mcp.Tool{
		Name:        "ls",
		Description: "List directory contents",
	}

	handler := func(ctx context.Context, req *mcp.CallToolRequest, input LsInput) (*mcp.CallToolResult, LsInput, error) {
		if input.Path == "" {
			return ErrorResult("path is required"), input, nil
		}

		result, err := StreamCommand(ctx, req, "ls", "-la", input.Path)
		if err != nil {
			return ErrorResult(err.Error()), input, nil
		}

		return FormatResult(result), input, nil
	}

	mcp.AddTool(server, tool, handler)
}

// BazelInput defines the input for the bazel tool
type BazelInput struct {
	Subcommand string `json:"subcommand" jsonschema_description:"Bazel subcommand (build or test)"`
	Target     string `json:"target" jsonschema_description:"Bazel target (e.g. //path/to:target)"`
}

// AllowedBazelSubcommands defines valid bazel subcommands
var AllowedBazelSubcommands = map[string]bool{
	"build": true,
	"test":  true,
}

// RegisterBazelTool registers the bazel tool with the server
func RegisterBazelTool(server *mcp.Server) {
	tool := &mcp.Tool{
		Name:        "bazel",
		Description: "Run bazel build or test commands",
	}

	handler := func(ctx context.Context, req *mcp.CallToolRequest, input BazelInput) (*mcp.CallToolResult, BazelInput, error) {
		if input.Subcommand == "" {
			return ErrorResult("subcommand is required"), input, nil
		}
		if !AllowedBazelSubcommands[input.Subcommand] {
			return ErrorResult("subcommand must be 'build' or 'test'"), input, nil
		}
		if input.Target == "" {
			return ErrorResult("target is required"), input, nil
		}

		result, err := StreamCommand(ctx, req, "bazel", input.Subcommand, input.Target)
		if err != nil {
			return ErrorResult(err.Error()), input, nil
		}

		return FormatResult(result), input, nil
	}

	mcp.AddTool(server, tool, handler)
}
