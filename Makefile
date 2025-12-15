.PHONY: build release test clean run inspect

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

clean:
	cargo clean

run: build
	./target/debug/command-runner-mcp-server-rust

inspect: build
	npx @modelcontextprotocol/inspector ./target/debug/command-runner-mcp-server-rust
