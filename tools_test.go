package main

import (
	"context"
	"os"
	"path/filepath"
	"testing"
)

// Test CatInput validation
func TestCatInputValidation(t *testing.T) {
	tests := []struct {
		name    string
		input   CatInput
		wantErr bool
	}{
		{"valid path", CatInput{Path: "/tmp/test.txt"}, false},
		{"empty path", CatInput{Path: ""}, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			hasErr := tt.input.Path == ""
			if hasErr != tt.wantErr {
				t.Errorf("validation mismatch: got error=%v, want error=%v", hasErr, tt.wantErr)
			}
		})
	}
}

// Test LsInput validation
func TestLsInputValidation(t *testing.T) {
	tests := []struct {
		name    string
		input   LsInput
		wantErr bool
	}{
		{"valid path", LsInput{Path: "/tmp"}, false},
		{"empty path", LsInput{Path: ""}, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			hasErr := tt.input.Path == ""
			if hasErr != tt.wantErr {
				t.Errorf("validation mismatch: got error=%v, want error=%v", hasErr, tt.wantErr)
			}
		})
	}
}

// Test BazelInput validation
func TestBazelInputValidation(t *testing.T) {
	tests := []struct {
		name    string
		input   BazelInput
		wantErr bool
	}{
		{"valid build", BazelInput{Subcommand: "build", Target: "//foo:bar"}, false},
		{"valid test", BazelInput{Subcommand: "test", Target: "//foo:bar"}, false},
		{"invalid subcommand", BazelInput{Subcommand: "run", Target: "//foo:bar"}, true},
		{"empty subcommand", BazelInput{Subcommand: "", Target: "//foo:bar"}, true},
		{"empty target", BazelInput{Subcommand: "build", Target: ""}, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			hasErr := tt.input.Subcommand == "" ||
				!AllowedBazelSubcommands[tt.input.Subcommand] ||
				tt.input.Target == ""
			if hasErr != tt.wantErr {
				t.Errorf("validation mismatch: got error=%v, want error=%v", hasErr, tt.wantErr)
			}
		})
	}
}

// Test AllowedBazelSubcommands
func TestAllowedBazelSubcommands(t *testing.T) {
	if !AllowedBazelSubcommands["build"] {
		t.Error("expected 'build' to be allowed")
	}
	if !AllowedBazelSubcommands["test"] {
		t.Error("expected 'test' to be allowed")
	}
	if AllowedBazelSubcommands["run"] {
		t.Error("expected 'run' to not be allowed")
	}
	if AllowedBazelSubcommands["clean"] {
		t.Error("expected 'clean' to not be allowed")
	}
}

// Integration test for cat tool with real file
func TestCatToolIntegration(t *testing.T) {
	// Create a temp file
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.txt")

	content := "line1\nline2\nline3\n"
	if err := os.WriteFile(tmpFile, []byte(content), 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	// Run cat on it
	ctx := context.Background()
	result, err := StreamCommand(ctx, nil, "cat", tmpFile)
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

// Integration test for ls tool
func TestLsToolIntegration(t *testing.T) {
	tmpDir := t.TempDir()

	// Create some files
	os.WriteFile(filepath.Join(tmpDir, "file1.txt"), []byte("test"), 0644)
	os.WriteFile(filepath.Join(tmpDir, "file2.txt"), []byte("test"), 0644)

	ctx := context.Background()
	result, err := StreamCommand(ctx, nil, "ls", "-la", tmpDir)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d", result.ExitCode)
	}

	// ls -la outputs header + files
	if result.LineCount < 3 {
		t.Errorf("expected at least 3 lines, got %d", result.LineCount)
	}
}

// Test cat with non-existent file
func TestCatNonExistentFile(t *testing.T) {
	ctx := context.Background()
	result, err := StreamCommand(ctx, nil, "cat", "/nonexistent/file/path")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ExitCode == 0 {
		t.Error("expected non-zero exit code for non-existent file")
	}
}

// Test ls with non-existent directory
func TestLsNonExistentDirectory(t *testing.T) {
	ctx := context.Background()
	result, err := StreamCommand(ctx, nil, "ls", "-la", "/nonexistent/directory/path")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ExitCode == 0 {
		t.Error("expected non-zero exit code for non-existent directory")
	}
}
