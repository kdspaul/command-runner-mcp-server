package main

import (
	"context"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

const (
	serverName    = "command-runner-mcp"
	serverVersion = "v0.2.0"
)

func main() {
	impl := &mcp.Implementation{
		Name:    serverName,
		Version: serverVersion,
	}
	server := mcp.NewServer(impl, nil)

	// Register tools
	RegisterCatTool(server)
	RegisterLsTool(server)
	RegisterBazelTool(server)

	// Graceful shutdown
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		log.Println("Shutting down...")
		cancel()
	}()

	// Run server
	log.Printf("Starting %s %s\n", serverName, serverVersion)
	if err := server.Run(ctx, &mcp.StdioTransport{}); err != nil && err != context.Canceled {
		log.Fatalf("Server error: %v", err)
	}
}
