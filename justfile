# egui-mcp development commands

# Show available commands
default:
    @just --list

# Run all checks (clippy + fmt)
check:
    cargo clippy --all-targets --all-features -- -D warnings
    cargo fmt --check

# Format code
fmt:
    cargo fmt

# Build all targets
build:
    cargo build --workspace

# Build release
release:
    cargo build --workspace --release

# Run tests
test:
    cargo test --workspace

# Run demo app
demo:
    cargo run -p demo-app

# Run MCP server
server:
    cargo run -p egui-mcp-server

# Clean build artifacts
clean:
    cargo clean
