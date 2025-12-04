# Binary name
BINARY_NAME := command-runner-mcp-server

# Go parameters
GOCMD := go
GOBUILD := $(GOCMD) build
GOTEST := $(GOCMD) test
GOCLEAN := $(GOCMD) clean
GOMOD := $(GOCMD) mod

# Build flags
LDFLAGS := -s -w

# Output directory for binaries
DIST_DIR := dist

# Detect current OS and architecture
GOOS := $(shell go env GOOS)
GOARCH := $(shell go env GOARCH)

.PHONY: all build test clean tidy fmt vet lint run \
        build-linux-amd64 build-linux-arm64 \
        build-darwin-amd64 build-darwin-arm64 \
        build-all-platforms

all: build

build:
	$(GOBUILD) -ldflags "$(LDFLAGS)" -o $(DIST_DIR)/$(GOOS)-$(GOARCH)/$(BINARY_NAME) .

# Linux builds
build-linux-amd64:
	GOOS=linux GOARCH=amd64 $(GOBUILD) -ldflags "$(LDFLAGS)" -o $(DIST_DIR)/linux-amd64/$(BINARY_NAME) .

build-linux-arm64:
	GOOS=linux GOARCH=arm64 $(GOBUILD) -ldflags "$(LDFLAGS)" -o $(DIST_DIR)/linux-arm64/$(BINARY_NAME) .

# macOS builds
build-darwin-amd64:
	GOOS=darwin GOARCH=amd64 $(GOBUILD) -ldflags "$(LDFLAGS)" -o $(DIST_DIR)/darwin-amd64/$(BINARY_NAME) .

build-darwin-arm64:
	GOOS=darwin GOARCH=arm64 $(GOBUILD) -ldflags "$(LDFLAGS)" -o $(DIST_DIR)/darwin-arm64/$(BINARY_NAME) .

# Build all platforms
build-all-platforms: build-linux-amd64 build-linux-arm64 build-darwin-amd64 build-darwin-arm64

test:
	$(GOTEST) -v ./...

test-coverage:
	$(GOTEST) -v -cover -coverprofile=coverage.out ./...
	$(GOCMD) tool cover -html=coverage.out -o coverage.html

clean:
	$(GOCLEAN)
	rm -f coverage.out coverage.html
	rm -rf $(DIST_DIR)

tidy:
	$(GOMOD) tidy

fmt:
	$(GOCMD) fmt ./...

vet:
	$(GOCMD) vet ./...

run: build
	./$(DIST_DIR)/$(GOOS)-$(GOARCH)/$(BINARY_NAME)
